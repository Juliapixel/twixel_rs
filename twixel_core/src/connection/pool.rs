use std::task::Poll;

use either::Either;
use futures_util::{FutureExt, Sink, SinkExt, Stream, future::select_all};
use hashbrown::HashMap;
use smallvec::SmallVec;

use crate::{
    auth::Auth,
    irc_message::{ToIrcMessage, builder::MessageBuilder, message::IrcMessage},
};

use super::{Connection, error::PoolError};

// current limit
const MAX_CHANNELS_PER_CONNECTION: usize = 100;

pub struct ConnectionPool {
    pool: Vec<Connection>,
    // relation between channel and connection index in the pool
    channels: HashMap<String, Option<usize>>,
    auth_info: Auth,
}

impl ConnectionPool {
    pub async fn new(
        channels: impl IntoIterator<Item = impl Into<String>>,
        auth: Auth,
    ) -> Result<Self, PoolError> {
        let mut pool = Vec::new();
        let mut channel_list = HashMap::new();
        let channels: Vec<String> = channels.into_iter().map(|c| c.into()).collect();

        for (i, window) in channels.windows(MAX_CHANNELS_PER_CONNECTION).enumerate() {
            let mut conn = Connection::new(window, auth.clone());
            conn.start().await?;
            pool.push(conn);
            for channel in window.iter() {
                channel_list.insert(channel.to_owned(), Some(i));
            }
        }

        Ok(Self {
            pool,
            channels: channel_list,
            auth_info: auth,
        })
    }

    pub async fn part_channel(&mut self, channel_login: &str) -> Result<(), PoolError> {
        match self
            .channels
            .remove(channel_login)
            .flatten()
            .and_then(|c| self.pool.get_mut(c))
        {
            Some(conn) => {
                conn.part(channel_login).await?;
                Ok(())
            }
            None => Err(PoolError::ChannelNotFound(channel_login.into())),
        }
    }

    pub async fn join_channel(&mut self, channel_login: &str) -> Result<(), PoolError> {
        match self
            .pool
            .iter_mut()
            .enumerate()
            .find(|c| c.1.get_channel_count() < MAX_CHANNELS_PER_CONNECTION)
        {
            Some((idx, conn)) => {
                conn.join(channel_login).await?;
                self.channels.insert(channel_login.into(), Some(idx));
                Ok(())
            }
            None => {
                let mut conn =
                    Connection::new(core::iter::once(channel_login), self.auth_info.clone());
                conn.start().await?;
                self.pool.push(conn);
                self.channels
                    .insert(channel_login.into(), Some(self.pool.len() - 1));

                Ok(())
            }
        }
    }

    pub fn get_conn_idx(&self, channel_login: &str) -> Option<usize> {
        self.channels.get(channel_login).copied().flatten()
    }

    pub async fn send_to_channel(&mut self, message: &str, channel: &str) -> Result<(), PoolError> {
        let conn_idx = self
            .channels
            .get(channel)
            .ok_or(PoolError::ChannelNotFound(channel.into()))?
            .ok_or(PoolError::NoConnectionAssigned(channel.into()))?;

        let conn = self
            .pool
            .get_mut(conn_idx)
            .expect("requested channel not in pool!");
        conn.send(MessageBuilder::privmsg(channel, message)).await?;

        Ok(())
    }

    pub async fn restart_connection(&mut self, index: usize) -> Result<(), PoolError> {
        let pool_len = self.pool.len();
        self.pool
            .get_mut(index)
            .ok_or(PoolError::IndexOutOfBounds(index, pool_len))?
            .restart()
            .await?;
        Ok(())
    }

    pub async fn send_to_connection(
        &mut self,
        msg: impl ToIrcMessage,
        idx: usize,
    ) -> Result<(), PoolError> {
        let len = self.pool.len();
        let conn = self
            .pool
            .get_mut(idx)
            .ok_or(PoolError::IndexOutOfBounds(idx, len))?;
        conn.send(msg).await?;
        Ok(())
    }
}

impl Stream for ConnectionPool {
    type Item = Result<(SmallVec<[IrcMessage<'static>; 4]>, usize), PoolError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.pool.is_empty() {
            return Poll::Ready(Some(Err(PoolError::NoConnections)));
        }

        if let Poll::Ready((received, idx, _futures)) =
            select_all(self.pool.iter_mut().map(|c| c.receive().boxed())).poll_unpin(cx)
        {
            let received = received.map_err(Into::<PoolError>::into);
            Poll::Ready(Some(received.map(|r| (r, idx))))
        } else {
            Poll::Pending
        }
    }
}

impl<'a> Sink<(Either<usize, &str>, MessageBuilder<'a>)> for ConnectionPool {
    type Error = PoolError;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut readied = 0;
        for i in self.pool.iter_mut() {
            match futures_util::ready!(i.poll_ready_unpin(cx)) {
                Ok(()) => readied += 1,
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }
        if readied == 0 {
            Poll::Ready(Err(PoolError::NoConnections))
        } else if readied == self.pool.len() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        (target, msg): (Either<usize, &str>, MessageBuilder<'a>),
    ) -> Result<(), Self::Error> {
        let conn_idx = match target {
            Either::Left(idx) => idx,
            Either::Right(chan) => match self.get_conn_idx(chan) {
                Some(idx) => idx,
                None => return Err(PoolError::ChannelNotFound(chan.to_string())),
            },
        };
        let Some(conn) = self.pool.get_mut(conn_idx) else {
            return Err(PoolError::IndexOutOfBounds(conn_idx, self.pool.len()));
        };
        conn.start_send_unpin(msg).map_err(Into::into)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut flushed = 0;
        for i in self.pool.iter_mut() {
            match futures_util::ready!(i.poll_flush_unpin(cx)) {
                Ok(()) => flushed += 1,
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }
        if flushed == 0 {
            Poll::Ready(Err(PoolError::NoConnections))
        } else if flushed == self.pool.len() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut closed = 0;
        for i in self.pool.iter_mut() {
            match futures_util::ready!(i.poll_close_unpin(cx)) {
                Ok(()) => closed += 1,
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }
        if closed == 0 {
            Poll::Ready(Err(PoolError::NoConnections))
        } else if closed == self.pool.len() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}
