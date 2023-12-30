use std::{sync::{Arc, Mutex}, pin::Pin, ops::DerefMut, time::Duration};

use futures_util::{StreamExt, SinkExt, FutureExt};
use log::{warn, debug, info};
use thiserror::Error;
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use tokio_tungstenite::{tungstenite::{Message, protocol::WebSocketConfig}, WebSocketStream, MaybeTlsStream};

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
    ReceiveError(#[from] ConnectionReceiveError)
}

type Websocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// handles the interface between the raw `Socket` and the `TwitchIrcClient`
pub(crate) struct Connection {
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

    pub async fn start(&mut self) -> Result<(), ConnectionStartError> {
        if self.socket.is_some() {
            warn!("tried starting connection when it was already started");
            return Err(ConnectionStartError::AlreadyStarted)
        }
        let (new_socket, _resp) = tokio_tungstenite::connect_async(TWITCH_IRC_URL).await?;

        self.socket = Some(new_socket);

        let client_info = self.client_info.lock().unwrap();
        let auth_messages = client_info.auth.into_commands();
        let join_msg = client_info.self_info.get_join_message();
        drop(client_info);

        self.send(auth_messages.0).await?;
        self.send(auth_messages.1).await?;

        if let Some(join_msg) = join_msg {
            self.send(join_msg).await?;
        }

        Ok(())
    }

    pub async fn receive(&mut self) -> Result<RawIrcMessage, ConnectionReceiveError> {
        if let Some(socket) = &mut self.socket {
            // TODO: treat a connection closed by the host
            // TODO: handle received system messages: RECONNECT, USERSTATE, ROOMSTATE, GLOBALUSERSTATE, PING
            // FIXME: don't unwrap here
            Ok(RawIrcMessage::try_from(socket.next().await.unwrap().unwrap().to_text().unwrap())?)
        } else {
            Err(ConnectionReceiveError::NotStarted)
        }
    }

    pub async fn send(&mut self, message: OwnedIrcMessage) -> Result<(), ConnectionSendError>{
        if let Some(socket) = &mut self.socket {
            // TODO: treat a connection closed by the host
            socket.send(Message::Text(message.to_string())).await?;
            Ok(())
        } else {
            Err(ConnectionSendError::NotStarted)
        }
    }
}
