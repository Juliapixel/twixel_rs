use std::{sync::{Arc, Mutex}, pin::Pin, ops::DerefMut, time::Duration};

use futures_util::{StreamExt, SinkExt, FutureExt};
use log::{warn, debug, info};
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream, MaybeTlsStream};

use crate::{user::ClientInfo, irc_message::message::{IrcMessage, IrcMessageFormatter}};

const TWITCH_IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";

/// handles the interface between the raw `Socket` and the `TwitchIrcClient`
pub(crate) struct Connection {
    socket: Option<Socket>,
    // what the fuck did i mean by this
    received: Option<Receiver<String>>,
    // and this
    sender: Option<Sender<String>>,
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
        let (mut socket, sender) = Socket::new().await;
        let (tx, received) = tokio::sync::mpsc::channel(64);
        self.received = Some(received);
        self.sender = Some(sender);
        info!("Twitch connection loop started");
        // FIXME: doesn't work, since all socket restarting logic has been moved
        // from `Socket` to `Connection`
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
                                tx.send(received.to_string()).await.unwrap();
                            }
                        }
                    },
                    Err(_) => socket.restart_socket().await,
                };
            }
        });
    }

    pub async fn send(&mut self, msg: IrcMessage) {
        if let Some(sender) = &self.sender {
            sender.send(msg.to_string(IrcMessageFormatter::Client)).await.unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    pub async fn receive(&mut self) -> IrcMessage {
        if let Some(received) = &mut self.received {
            // FIXME: do not unwrap here
            received.recv().await.unwrap().as_str().try_into().unwrap()
        } else {
            panic!();
        }
    }

    pub async fn join_channel(&mut self, channel: String) {
        if let Some(sender) = &self.sender {
            sender.blocking_send(format!("JOIN #{}", &channel)).unwrap();
            self.client_info.lock().unwrap().self_info.join_channel(channel);
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    pub async fn leave_channel(&mut self, channel: &str) {
        self.client_info.lock().unwrap().self_info.leave_channel(channel);
        if let Some(sender) = &self.sender {
            sender.blocking_send(format!("PART #{}", channel)).unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }

    async fn send_pong(&mut self) {
        if let Some(sender) = &self.sender {
            sender.blocking_send("PONG :tmi.twitch.tv".to_string()).unwrap();
        } else {
            warn!("tried to send to socket while it's not running!");
        }
    }
}

struct Socket {
    stream: Pin<Box<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    outgoing: tokio::sync::mpsc::Receiver<String>,
}

#[derive(Debug)]
enum ReadReceiveError {
    NoMessage,
    TungsteniteError(tokio_tungstenite::tungstenite::Error)
}

impl Socket {
    pub async fn new() -> (Self, tokio::sync::mpsc::Sender<String>) {
        let stream = Self::start_socket().await;
        let (send, recv) = tokio::sync::mpsc::channel(64);
        (
            Self {
                stream: stream,
                outgoing: recv,
            },
            send
        )
    }

    async fn get_new_socket() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        let mut delay = Duration::from_secs(1);
        loop {
            match tokio_tungstenite::connect_async(TWITCH_IRC_URL).await {
                Ok(o) => {
                    return o.0;
                },
                Err(_) => {
                    warn!(
                        "failed to connect to twitch servers! retrying... in {} second(s)",
                        delay.as_secs()
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                    continue;
                },
            }
        }
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
                let send = sink.send(Message::Text(msg)).await;
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
        self.stream = Self::start_socket().await;
        info!("connection restarted.");
    }

    async fn start_socket() -> Pin<Box<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>> {
        let new_stream = Self::get_new_socket().await;
        return Box::pin(Some(new_stream));
    }
}
