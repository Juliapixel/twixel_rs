use std::sync::{Arc, Mutex};

use futures_util::{StreamExt, SinkExt};
use log::{warn, debug};
use smallvec::SmallVec;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream, MaybeTlsStream};

use crate::{user::ClientInfo, irc_message::{raw::RawIrcMessage, owned::OwnedIrcMessage, command::IrcCommand, error::RawIrcMessageParseError}};

const TWITCH_IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";

#[derive(Debug, Error)]
pub enum ConnectionStartError {
    #[error("this connection had already been started!")]
    AlreadyStarted,
    #[error("failed to send initial messages to twitch:\n{0}")]
    SendError(#[from] ConnectionSendError),
    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error)
}

#[derive(Debug, Error)]
pub enum ConnectionReceiveError {
    #[error("this Connection has not been started yet")]
    NotStarted,
    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("the received message from the websocket was not a valid IRC message:\n {0}")]
    NotValidMessage(#[from] RawIrcMessageParseError)
}

#[derive(Debug, Error)]
pub enum ConnectionSendError {
    #[error("this Connection has not been started yet")]
    NotStarted,
    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error(transparent)]
    StartError(#[from] ConnectionStartError),
    #[error(transparent)]
    ReceiveError(#[from] ConnectionReceiveError),
    #[error(transparent)]
    SendError(#[from] ConnectionSendError)
}

type Websocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// handles the interface between the raw `Socket` and the `TwitchIrcClient`
pub struct Connection {
    socket: Option<Websocket>,
    client_info: Arc<Mutex<ClientInfo>>
}

// TODO: add logging
impl Connection {
    pub fn new(client_info: Arc<Mutex<ClientInfo>>) -> Self {
        Self {
            socket: None,
            client_info
        }
    }

    pub async fn start(&mut self) -> Result<(), ConnectionError> {
        if self.socket.is_some() {
            warn!("tried starting connection when it was already started");
            return Err(ConnectionStartError::AlreadyStarted)?
        }
        let (new_socket, _resp) = tokio_tungstenite::connect_async(TWITCH_IRC_URL)
            .await
            .map_err(|e| ConnectionStartError::TungsteniteError(e))?;

        self.socket = Some(new_socket);

        let client_info = self.client_info.lock().unwrap();
        let auth_messages = client_info.auth.into_commands();
        let join_msg = client_info.self_info.get_join_message();
        drop(client_info);

        let cap_req = OwnedIrcMessage {
            tags: None,
            prefix: None,
            command: IrcCommand::Cap,
            params: vec![
                "REQ".into(),
                ":twitch.tv/commands twitch.tv/tags".into()
            ],
        };

        if let Some(join_msg) = join_msg {
            self.send_batched(&[auth_messages.0, auth_messages.1, cap_req, join_msg]).await?;
        } else {
            self.send_batched(&[auth_messages.0, auth_messages.1, cap_req]).await?;
        }

        Ok(())
    }

    /// receives and automatically handles keepalive and other system messages
    pub async fn receive(&mut self) -> Result<SmallVec<[RawIrcMessage; 4]>, ConnectionError> {
        if let Some(socket) = &mut self.socket {
            // TODO: treat a connection closed by the host
            // TODO: handle received system messages: RECONNECT, USERSTATE, ROOMSTATE, GLOBALUSERSTATE, PING
            // FIXME: don't unwrap here
            let received_text = socket.next().await.unwrap().unwrap().to_text().unwrap().to_string();

            let mut received_messages = SmallVec::new();
            let mut pos = 0;
            for i in memchr::memchr_iter(b'\n', received_text.as_bytes()) {
                let parsed = RawIrcMessage::try_from(&received_text[pos..=i]).map_err(|e| ConnectionReceiveError::NotValidMessage(e))?;
                if parsed.command == IrcCommand::Ping {
                    self.send(OwnedIrcMessage::pong(parsed.get_param(0).unwrap().into())).await?;
                }
                received_messages.push(parsed);
                pos = i + 1;
            }
            Ok(received_messages)
        } else {
            Err(ConnectionReceiveError::NotStarted)?
        }
    }

    pub async fn send(&mut self, message: OwnedIrcMessage) -> Result<(), ConnectionSendError> {
        if let Some(socket) = &mut self.socket {
            let out = message.to_string();
            debug!("sent: {}", out.trim());
            // TODO: treat a connection closed by the host
            socket.send(Message::Text(out)).await?;
            Ok(())
        } else {
            Err(ConnectionSendError::NotStarted)
        }
    }

    pub async fn send_batched(&mut self, messages: &[OwnedIrcMessage]) -> Result<(), ConnectionSendError> {
        if let Some(socket) = &mut self.socket {
            for i in messages {
                let out = i.to_string();
                debug!("sent: {}", if i.command == IrcCommand::Pass { "*redacted for privacy*" } else { &out } );
                // TODO: treat a connection closed by the host
                socket.feed(Message::Text(out)).await?;
            }
            socket.flush().await?;
            Ok(())
        } else {
            Err(ConnectionSendError::NotStarted)
        }
    }
}
