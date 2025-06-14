use std::task::Poll;

use error::ConnectionError;
use futures_util::{SinkExt, StreamExt};
use hashbrown::HashSet;
use log::{debug, warn};
use smallvec::SmallVec;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as WsMessage};

pub mod pool;
// #[cfg(feature = "unstable")]
// pub mod stream;

pub use pool::ConnectionPool;

use crate::{
    auth::Auth,
    irc_message::{
        ToIrcMessage, builder::MessageBuilder, command::IrcCommand, message::IrcMessage,
    },
};

pub mod error {
    use thiserror::Error;
    use tokio_tungstenite::tungstenite::{Error as TungsteniteError, error::ProtocolError};

    use crate::irc_message::error::IrcMessageParseError;

    #[derive(Debug, Error)]
    pub enum ConnectionError {
        #[error("this Connection has not been started yet")]
        NotStarted,
        #[error("this Connection has already already started")]
        AlreadyStarted,
        #[error("this Connection has been closed")]
        Closed,
        #[error(transparent)]
        TungsteniteError(TungsteniteError),
        #[error("the received message from the websocket was not a valid IRC message:\n {0}")]
        InvalidMessage(#[from] IrcMessageParseError),
    }

    #[derive(Debug, Error)]
    pub enum PoolError {
        #[error(transparent)]
        ConnectionError(#[from] ConnectionError),
        #[error("The requested channel was not found {0}")]
        ChannelNotFound(String),
        #[error("requested channel didn't have a connection assigned to it {0}")]
        NoConnectionAssigned(String),
        #[error("requested index is {0} but length is {1}")]
        IndexOutOfBounds(usize, usize),
        #[error("there are no connections to receive from")]
        NoConnections,
    }

    impl From<TungsteniteError> for ConnectionError {
        fn from(value: TungsteniteError) -> Self {
            match value {
                TungsteniteError::AlreadyClosed
                | TungsteniteError::ConnectionClosed
                | TungsteniteError::Protocol(ProtocolError::InvalidCloseSequence) => Self::Closed,
                e => Self::TungsteniteError(e),
            }
        }
    }
}

const TWITCH_IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";

type Websocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// handles the interface between the raw `Socket` and the `TwitchIrcClient`
pub struct Connection {
    socket: Option<Websocket>,
    state: ConnectionState,
    channel_list: HashSet<String>,
    auth_info: Auth,
}

pub enum ConnectionState {
    /// Connection is closed
    Closed,
    /// Connection is open but no authentication has been acknowledged
    StartedUnauthed,
    /// Connection is open and ready to receive
    Working,
}

// TODO: add logging
impl Connection {
    pub fn new(channels: impl IntoIterator<Item = impl Into<String>>, auth: Auth) -> Self {
        Self {
            socket: None,
            state: ConnectionState::Closed,
            channel_list: channels.into_iter().map(|i| i.into()).collect(),
            auth_info: auth,
        }
    }

    pub async fn start(&mut self) -> Result<(), ConnectionError> {
        if self.socket.is_some() {
            warn!("tried starting connection when it was already started");
            return Err(ConnectionError::AlreadyStarted)?;
        }

        let (new_socket, _resp) = tokio_tungstenite::connect_async(TWITCH_IRC_URL)
            .await
            .map_err(ConnectionError::TungsteniteError)?;

        self.socket = Some(new_socket);
        self.state = ConnectionState::StartedUnauthed;

        let (pass, nick) = self.auth_info.into_commands();

        let join_msg = {
            if !self.channel_list.is_empty() {
                Some(MessageBuilder::join(self.channel_list.iter()))
            } else {
                None
            }
        };

        let cap_req = MessageBuilder::cap_req();

        let (nick, pass) = (nick.to_owned(), pass.to_owned());

        self.feed(pass).await?;
        self.feed(nick).await?;
        self.feed(cap_req.to_owned()).await?;
        if let Some(join_msg) = join_msg {
            self.feed(join_msg.to_owned()).await?;
        }
        self.flush().await?;

        Ok(())
    }

    pub async fn restart(&mut self) -> Result<(), ConnectionError> {
        if let Some(mut socket) = self.socket.take() {
            socket.close(None).await?;
        }
        self.start().await
    }

    pub async fn join(&mut self, channel: &str) -> Result<(), ConnectionError> {
        if self.channel_list.insert(channel.into()) {
            self.send(MessageBuilder::join(std::iter::once(&channel)))
                .await?;
        }
        Ok(())
    }

    pub async fn part(&mut self, channel: &str) -> Result<(), ConnectionError> {
        if self.channel_list.remove(channel) {
            self.send(MessageBuilder::part(std::iter::once(channel)))
                .await?;
        }
        Ok(())
    }

    /// receives twitch messages directly
    pub async fn receive(&mut self) -> Result<SmallVec<[IrcMessage<'static>; 4]>, ConnectionError> {
        if let Some(socket) = &mut self.socket {
            let received_msg = socket.next().await.ok_or(ConnectionError::Closed)??;

            let mut received = SmallVec::new();

            for recv in IrcMessage::from_ws_message(&received_msg) {
                match recv {
                    Ok(r) => {
                        if r.get_command() == IrcCommand::AuthSuccessful {
                            self.state = ConnectionState::Working;
                        }
                        received.push(r.to_static())
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(received)
        } else {
            Err(ConnectionError::NotStarted)
        }
    }

    pub async fn send(&mut self, message: impl ToIrcMessage) -> Result<(), ConnectionError> {
        if let Some(socket) = &mut self.socket {
            let command = message.get_command();
            let out = message.to_message();
            debug!(
                "sent: {:?}",
                if command == IrcCommand::Pass {
                    "[user token redacted]"
                } else {
                    out.trim()
                }
            );
            socket.send(WsMessage::Text(out.into())).await?;
            Ok(())
        } else {
            Err(ConnectionError::NotStarted)
        }
    }

    pub async fn send_batched(
        &mut self,
        messages: impl IntoIterator<Item = impl ToIrcMessage>,
    ) -> Result<(), ConnectionError> {
        if let Some(socket) = &mut self.socket {
            for i in messages {
                let cmd = i.get_command();
                let out = i.to_message();
                debug!(
                    "sent: {:?}",
                    if cmd == IrcCommand::Pass {
                        "[user token redacted]"
                    } else {
                        out.trim()
                    }
                );
                socket.feed(WsMessage::Text(out.into())).await?;
            }
            socket.flush().await?;
            Ok(())
        } else {
            Err(ConnectionError::NotStarted)
        }
    }

    pub fn get_channel_count(&self) -> usize {
        self.channel_list.len()
    }

    pub fn to_stream(self) -> impl futures_util::Stream {
        futures_util::stream::unfold(self, |mut state| async move {
            Some((state.receive().await, state))
        })
    }
}

impl futures_util::Stream for Connection {
    type Item = Result<SmallVec<[IrcMessage<'static>; 4]>, ConnectionError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let Some(socket) = self.socket.as_mut() else {
            return Poll::Ready(None);
        };
        let ready = futures_util::ready!(socket.poll_next_unpin(cx));
        match ready {
            Some(Ok(recv)) => {
                let mut received = SmallVec::new();
                for msg in IrcMessage::from_ws_message(&recv) {
                    match msg {
                        Ok(msg) => received.push(msg.to_static()),
                        Err(e) => return Poll::Ready(Some(Err(e.into()))),
                    }
                }

                Poll::Ready(Some(Ok(received)))
            }
            Some(Err(e)) => Poll::Ready(Some(Err(e.into()))),
            None => todo!(),
        }
    }
}

impl<'a> futures_util::Sink<MessageBuilder<'a>> for Connection {
    type Error = ConnectionError;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.socket
            .as_mut()
            .ok_or(ConnectionError::NotStarted)?
            .poll_ready_unpin(cx)
            .map_err(Into::into)
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: MessageBuilder<'a>,
    ) -> Result<(), Self::Error> {
        self.socket
            .as_mut()
            .ok_or(ConnectionError::NotStarted)?
            .start_send_unpin(WsMessage::Text(item.build().into()))
            .map_err(Into::into)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.socket
            .as_mut()
            .ok_or(ConnectionError::NotStarted)?
            .poll_flush_unpin(cx)
            .map_err(Into::into)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.socket
            .as_mut()
            .ok_or(ConnectionError::NotStarted)?
            .poll_close_unpin(cx)
            .map_err(Into::into)
    }
}
