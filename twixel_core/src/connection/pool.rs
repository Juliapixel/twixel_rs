use std::task::Poll;

use futures_util::{future::select_all, FutureExt, Stream};
use hashbrown::HashMap;
use smallvec::SmallVec;

use crate::{
    auth::Auth,
    irc_message::{builder::MessageBuilder, message::IrcMessage, ToIrcMessage},
    user::ChannelInfo,
};

use super::{error::PoolError, Connection};

// idk
const MAX_CHANNELS_PER_CONNECTION: usize = 50;

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
            for channel in window.into_iter() {
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
        match self.channels.remove(channel_login).flatten().and_then(|c| self.pool.get_mut(c)) {
            Some(conn) => {
                conn.part(channel_login).await?;
                Ok(())
            },
            None => {
                Err(PoolError::ChannelNotFound(channel_login.into()))
            },
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
