use std::{collections::VecDeque, task::Poll};

use error::ConnectionError;
use futures_util::{Sink, SinkExt, Stream, StreamExt, stream::FusedStream};
use hashbrown::HashSet;
use log::{debug, warn};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as WsMessage};

pub mod pool;

pub use pool::ConnectionPool;

use crate::{
    auth::AuthProvider,
    irc_message::{
        ToIrcMessage, builder::MessageBuilder, command::IrcCommand, message::IrcMessage,
    },
};

/// Error types associated with [Connection] and related operations
pub mod error {
    use thiserror::Error;
    use tokio_tungstenite::tungstenite::{Error as TungsteniteError, error::ProtocolError};

    use crate::irc_message::error::IrcMessageParseError;

    /// [Connection](super::Connection) errors
    #[derive(Debug, Error)]
    pub enum ConnectionError {
        /// A method that requires a started connection was called
        #[error("this Connection has not been started yet")]
        NotStarted,
        /// An already started [Connection] was attempted to be started
        #[error("this Connection has already already started")]
        AlreadyStarted,
        /// A closed [Connection] was read/written to
        #[error("this Connection has been closed")]
        Closed,
        /// An Error in the `tokio_tungstenite` websocket library
        #[error(transparent)]
        TungsteniteError(TungsteniteError),
        /// An invalid IRCv3 message was received from the websocket
        #[error("the received message from the websocket was not a valid IRC message:\n {0}")]
        InvalidMessage(#[from] IrcMessageParseError),
        /// No content was received from the underlying websocket connection
        #[error("the Connection received a websocket message, but no valid content was found")]
        NoMessage,
    }

    /// [ConnectionPool](super::pool::ConnectionPool) errors
    #[derive(Debug, Error)]
    pub enum PoolError {
        /// An error related to an internal [Connection](super::Connection)
        #[error(transparent)]
        ConnectionError(#[from] ConnectionError),
        /// A channel interaction was requested for a channel that was not joined
        #[error("The requested channel was not found {0}")]
        ChannelNotFound(String),
        /// A requested channel didn't have a connection assigned to it
        #[error("requested channel didn't have a connection assigned to it {0}")]
        NoConnectionAssigned(String),
        /// Tried to operate on a [Connection](super::Connection) by index but the
        /// index was out of bounds
        #[error("requested index is {0} but length is {1}")]
        IndexOutOfBounds(usize, usize),
        /// There are no connections to receive from
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
pub struct Connection<A: AuthProvider> {
    socket: Option<Websocket>,
    state: ConnectionState,
    channel_list: HashSet<String>,
    buffer: VecDeque<Result<IrcMessage, ConnectionError>>,
    auth_info: Box<A>,
}

/// State of the [Connection]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    /// Connection is closed
    Closed,
    /// Connection is open but no authentication has been acknowledged
    StartedUnauthed,
    /// Connection is open and ready to receive
    Working,
}

// TODO: add logging
impl<A: AuthProvider> Connection<A> {
    /// Create a new [Connection] that joins `channels` upon being started
    pub fn new(channels: impl IntoIterator<Item = impl Into<String>>, auth: A) -> Self {
        Self {
            socket: None,
            state: ConnectionState::Closed,
            channel_list: channels.into_iter().map(|i| i.into()).collect(),
            buffer: VecDeque::new(),
            auth_info: Box::new(auth),
        }
    }

    /// The state of this connection
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Connects to the IRC websocket and sends `JOIN` messages for added channels.
    ///
    /// Errors if the connection is already started.
    pub async fn start(&mut self) -> Result<(), ConnectionError> {
        if self.socket.is_some() {
            warn!("tried starting connection when it was already started");
            return Err(ConnectionError::AlreadyStarted);
        }

        let (new_socket, _resp) = tokio_tungstenite::connect_async(TWITCH_IRC_URL)
            .await
            .map_err(ConnectionError::TungsteniteError)?;

        self.socket = Some(new_socket);
        self.state = ConnectionState::StartedUnauthed;

        let (pass, nick) = self.auth_info.get_commands();

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
        <Self as SinkExt<MessageBuilder>>::flush(self).await?;

        self.state = ConnectionState::Working;

        Ok(())
    }

    /// Closes the websocket and restarts the connection.
    pub async fn restart(&mut self) -> Result<(), ConnectionError> {
        if let Some(mut socket) = self.socket.take() {
            socket.close(None).await?;
        }
        self.start().await
    }

    /// Immediately sends `JOIN` message if the connection has been started, otherwise
    /// sends it when [Connection::start] is called
    pub async fn join(&mut self, channel: &str) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::Working {
            self.channel_list.insert(channel.into());
            return Ok(());
        } else if self.channel_list.insert(channel.into()) {
            self.send(MessageBuilder::join(std::iter::once(&channel)))
                .await?;
        }
        Ok(())
    }

    /// Sends `PART` message if the connection has been started, otherwise
    /// removes it from channels joined when [Connection::start] is called
    pub async fn part(&mut self, channel: &str) -> Result<(), ConnectionError> {
        if self.state != ConnectionState::Working {
            self.channel_list.remove(channel);
            return Ok(());
        } else if self.channel_list.remove(channel) {
            self.send(MessageBuilder::part(std::iter::once(channel)))
                .await?;
        }
        Ok(())
    }

    /// Receives a single new message from Twitch. Multi-message websocket messages
    /// have their IRC messages buffered and are returned immediately upon subsequent calls
    /// to this function.
    pub async fn receive(&mut self) -> Result<IrcMessage, ConnectionError> {
        if let Some(next) = self.buffer.pop_front() {
            log::trace!(
                "Received new message: {:?}",
                next.as_ref().map(|i| i.inner())
            );
            return next;
        }

        if let Some(socket) = &mut self.socket {
            let received_msg = socket.next().await.ok_or(ConnectionError::Closed)??;

            let mut msgs =
                IrcMessage::from_ws_message(&received_msg).map(|n| n.map_err(Into::into));

            let next = msgs.next().ok_or(ConnectionError::NoMessage)?;

            self.buffer.extend(msgs);

            log::trace!(
                "Received new message: {:?}",
                next.as_ref().map(|i| i.inner())
            );
            next
        } else {
            Err(ConnectionError::NotStarted)
        }
    }

    /// Immediately sends an IRC message to Twitch
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

    /// Immediately sends many IRC messages to Twitch. This method should be
    /// preferred to using [send](Connection::send) when many messages must be sent
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

    /// Number of channels added to this [Connection]
    pub fn get_channel_count(&self) -> usize {
        self.channel_list.len()
    }
}

impl<A: AuthProvider> FusedStream for Connection<A> {
    fn is_terminated(&self) -> bool {
        self.socket.as_ref().is_some_and(|s| s.is_terminated())
    }
}

impl<A: AuthProvider> Stream for Connection<A> {
    type Item = Result<IrcMessage, ConnectionError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let Some(next) = self.buffer.pop_front() {
            log::trace!(
                "Received new message: {:?}",
                next.as_ref().map(|i| i.inner())
            );
            return Poll::Ready(Some(next));
        }
        let Some(socket) = self.socket.as_mut() else {
            return Poll::Ready(Some(Err(ConnectionError::NotStarted)));
        };
        let ready = futures_util::ready!(socket.poll_next_unpin(cx));
        match ready {
            Some(Ok(recv)) => {
                let mut msgs = IrcMessage::from_ws_message(&recv).map(|n| n.map_err(Into::into));

                let next = msgs.next().ok_or(ConnectionError::NoMessage)?;

                self.buffer.extend(msgs);

                log::trace!(
                    "Received new message: {:?}",
                    next.as_ref().map(|i| i.inner())
                );
                Poll::Ready(Some(next))
            }
            Some(Err(e)) => Poll::Ready(Some(Err(e.into()))),
            None => Poll::Ready(Some(Err(ConnectionError::Closed))),
        }
    }
}

impl<T: ToIrcMessage, A: AuthProvider> Sink<T> for Connection<A> {
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

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.socket
            .as_mut()
            .ok_or(ConnectionError::NotStarted)?
            .start_send_unpin(WsMessage::Text(item.to_message().into()))
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
