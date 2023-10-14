use std::{sync::{Arc, Mutex}, pin::Pin, ops::DerefMut};

use futures_util::{StreamExt, SinkExt, FutureExt};
use log::{warn, debug, info};
use rand::Rng;
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream, MaybeTlsStream};

use crate::irc_message::IRCMessage;
use crate::user::SelfStatus;

const TWITCH_IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";

pub(crate) struct Connection {
    socket: Option<Socket>,
    received: Option<Receiver<Message>>,
    sender: Option<Sender<Message>>,
    client_info: Arc<Mutex<ClientInfo>>
}

impl Connection {
    pub async fn new(client_info: ClientInfo) -> Self {
        Connection {
            socket: None,
            received: None,
            sender: None,
            client_info: Arc::new(Mutex::new(client_info)),
        }
    }

    pub async fn run(&mut self) {
        let (mut socket, sender) = Socket::new(self.client_info.clone()).await;
        let (tx, received) = tokio::sync::mpsc::channel(64);
        self.received = Some(received);
        self.sender = Some(sender);
        info!("Twitch connection loop started");
        tokio::spawn(async move {
            loop {
                let (status, msg) = socket.receive_or_send().await;
                match status {
                    Ok(_) => {
                        if let Some(received) = msg {
                            if received.to_text().unwrap().trim() == "PING :tmi.twitch.tv" {
                                if let Some(stream) = socket.stream.deref_mut() {
                                    debug!("sending keepalive message");
                                    stream.send(Message::Text("PONG :tmi.twitch.tv".to_string())).await.unwrap();
                                }
                            } else {
                                tx.send(received).await.unwrap();
                            }
                        }
                    },
                    Err(_) => socket.restart_socket().await,
                };
            }
        });
    }

    pub async fn send(&mut self, msg: IRCMessage) {
        if let Some(sender) = &self.sender {
            sender.send(Message::Text(msg.to_string(crate::irc_message::IRCMessageFormatter::Client))).await.unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    pub async fn receive(&mut self) -> IRCMessage {
        if let Some(received) = &mut self.received {
            received.recv().await.unwrap().to_text().unwrap().try_into().unwrap()
        } else {
            panic!();
        }
    }

    pub async fn join_channel(&mut self, channel: String) {
        if let Some(sender) = &self.sender {
            sender.blocking_send(Message::Text(format!("JOIN #{}", &channel))).unwrap();
            self.client_info.lock().unwrap().self_info.join_channel(channel);
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    pub async fn leave_channel(&mut self, channel: &str) {
        self.client_info.lock().unwrap().self_info.leave_channel(channel);
        if let Some(sender) = &self.sender {
            sender.blocking_send(Message::Text(format!("PART #{}", channel))).unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    async fn send_pong(&mut self) {
        if let Some(sender) = &self.sender {
            sender.blocking_send(Message::Text("PONG :tmi.twitch.tv".to_string())).unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }
}

struct Socket {
    stream: Pin<Box<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    outgoing: tokio::sync::mpsc::Receiver<Message>,
    client_info: Arc<Mutex<ClientInfo>>
}

enum ReadReceiveError {
    NoMessage,
    TungsteniteError(tokio_tungstenite::tungstenite::Error)
}

impl Socket {
    async fn get_new_socket() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        loop {
            match tokio_tungstenite::connect_async(TWITCH_IRC_URL).await {
                Ok(o) => {
                    return o.0;
                },
                Err(_) => {
                    warn!("failed to connect to twitch servers! retrying...");
                    continue;
                },
            }
        }

    }

    pub async fn new(client_info: Arc<Mutex<ClientInfo>>) -> (Self, tokio::sync::mpsc::Sender<Message>) {
        let stream = Self::start_socket(client_info.clone()).await;
        let (send, recv) = tokio::sync::mpsc::channel(64);
        (
            Self {
                stream: stream,
                outgoing: recv,
                client_info: client_info,
            },
            send
        )
    }

    pub async fn receive_or_send(&mut self) -> (Result<(), ReadReceiveError>, Option<Message>) {
        let (mut sink, mut stream) = self.stream.take().unwrap().split();
        let received = futures_util::select! {
            recv = stream.next().fuse() => {
                match recv {
                    Some(received) => {
                        match received {
                            Ok(ok) => (Ok(()), Some(ok)),
                            Err(e) => {
                                (Err(ReadReceiveError::TungsteniteError(e)), None)
                            }
                        }
                    },
                    None => {
                        (Err(ReadReceiveError::NoMessage), None)
                    }
                }
            },
            to_send = self.outgoing.recv().fuse() => {
                let msg = to_send.unwrap();
                debug!("sent: {}", &msg);
                let send = sink.send(msg).await;
                match send {
                    Ok(_) => (Ok(()), None),
                    Err(e) => (Err(ReadReceiveError::TungsteniteError(e)), None)
                }
            },
        };
        self.stream = Box::pin(Some(sink.reunite(stream).unwrap()));
        return received;
    }

    async fn restart_socket(&mut self) {
        warn!("restarting connection to twitch servers.");
        self.stream = Self::start_socket(self.client_info.clone()).await;
        info!("connection restarted.");
    }

    async fn start_socket(client_info: Arc<Mutex<ClientInfo>>) -> Pin<Box<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>> {
        let mut new_stream = Self::get_new_socket().await;
        let initial_messages = client_info.lock().unwrap().get_initial_messages();
        for i in initial_messages {
            new_stream.send(i).await;
        }
        return Box::pin(Some(new_stream));
    }
}

#[derive(Debug, Default, Clone)]
pub enum Auth {
    OAuth{ username: String, token: String },
    #[default]
    Anonymous
}

impl Auth {
    fn into_commands(&self) -> (Message, Message) {
        match self {
            Self::OAuth { username, token } => {
                (
                    Message::text(String::from(format!("NICK {username}"))),
                    Message::text(String::from(format!("PASS {token}"))),
                )
            },
            Self::Anonymous => {
                let mut rng = rand::thread_rng();
                (
                    Message::text(String::from(format!("NICK justinfan{}", rng.gen_range(1..99999)))),
                    Message::text(String::from("PASS POGGERS"))
                )
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClientInfo {
    pub auth: Auth,
    pub self_info: SelfStatus,
}

impl ClientInfo {
    pub fn new(auth: Auth) -> Self {
        ClientInfo{
            auth: auth,
            self_info: SelfStatus::new()
        }
    }

    pub fn get_initial_messages(&self) -> Vec<Message> {
        let mut out = Vec::new();
        out.push(Message::Text(String::from("CAP REQ :twitch.tv/commands twitch.tv/tags")));
        let (nick, pass) = self.get_auth_commands();
        out.push(pass);
        out.push(nick);
        if let Some(join) = self.self_info.get_join_message() {
            out.push(join);
        }
        return out;
    }

    fn get_auth_commands(&self) -> (Message, Message) {
        self.auth.into_commands()
    }
}
