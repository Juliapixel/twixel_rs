use std::{sync::{Arc, Mutex}, net::TcpStream, collections::VecDeque};

use log::info;
use tungstenite::{WebSocket, stream::MaybeTlsStream};

const TWITCH_IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";

#[derive(Clone)]
pub(crate) struct Connection {
    socket: Socket,
    client_info: ClientInfo
}

impl Connection {
    pub fn new(client_info: ClientInfo) -> Self {
        Connection {
            socket: Socket::new(client_info.clone()),
            client_info: client_info,
        }
    }

    pub fn join_channel(&mut self, channel: &str) {
        self.client_info.channels.join_channel(channel);
        self.socket.send(&format!("JOIN #{}", channel));
    }

    pub fn leave_channel(&mut self, channel: &str) {
        self.client_info.channels.leave_channel(channel);
        self.socket.send(&format!("JOIN #{}", channel));
    }

    pub fn send_pong(&mut self) {
        self.socket.send("PONG :tmi.twitch.tv")
    }

    pub fn read(&mut self) -> tungstenite::Message {
        self.socket.receive_message()
    }

    pub fn send(&mut self, msg: &str) {
        self.socket.send(msg);
    }
}

#[derive(Clone)]
struct Socket {
    websocket: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    queue: Arc<Mutex<VecDeque<tungstenite::Message>>>,
    client_info: ClientInfo
}

impl Socket {
    fn get_new_socket() -> WebSocket<MaybeTlsStream<TcpStream>> {
        tungstenite::connect(TWITCH_IRC_URL).unwrap().0
    }

    pub fn new(client_info: ClientInfo) -> Self {
        Self {
            websocket: Arc::new(Mutex::new(Self::start_socket(client_info.clone()))),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            client_info: client_info,
        }
    }

    pub fn receive_message(&mut self) -> tungstenite::Message {
        loop {
            let mut slock = self.websocket.lock().unwrap();
            if let Ok(msg) = slock.read_message() {
                drop(slock);
                return msg;
            } else {
                info!("Reconnecting to twitch IRC Servers.");
                *slock = Self::start_socket(self.client_info.clone());
            }
        }

    }

    pub fn send(&mut self, message: &str) {
        let msg = tungstenite::Message::text(message);
        self.queue.lock().unwrap().push_back(msg);
        self.send_from_queue();
    }

    pub fn send_message(&mut self, message: tungstenite::Message) {
        self.queue.lock().unwrap().push_back(message);
    }

    fn send_from_queue(&mut self) {
        let mut lock  = self.queue.lock().unwrap();
        if let Some(msg) = lock.pop_front() {
            drop(lock);
            let mut slock = self.websocket.lock().unwrap();
            loop {
                loop {
                    let send_result = slock.write_message(msg.clone());
                    if send_result.is_err() {
                        info!("Reconnecting to twitch IRC Servers.");
                        *slock = Self::start_socket(self.client_info.clone());
                    } else {
                        break;
                    }
                }
                let flush_result = slock.write_pending();
                if flush_result.is_err() {
                    info!("Reconnecting to twitch IRC Servers.");
                    *slock = Self::start_socket(self.client_info.clone());
                } else {
                    break;
                }
            }
        }
    }

    #[allow(unused_must_use)]
    fn send_from_queue_unchecked(&mut self) {
        let mut lock  = self.queue.lock().unwrap();
        if let Some(msg) = lock.pop_front() {
            let mut slock = self.websocket.lock().unwrap();
            slock.write_message(msg);
            slock.write_pending();
        }
    }

    #[allow(unused_must_use)]
    fn start_socket(client_info: ClientInfo) -> WebSocket<MaybeTlsStream<TcpStream>> {
        let mut new_socket = Self::get_new_socket();
        let initial_messages = client_info.get_initial_messages();
        for i in initial_messages {
            new_socket.write_message(i);
            new_socket.write_pending();
        }
        return new_socket;
    }
}

#[derive(Clone)]
pub(crate) struct ClientInfo {
    username: Arc<Mutex<String>>,
    auth_token: Arc<Mutex<String>>,
    pub channels: Channels,
}

impl ClientInfo {
    pub fn new(username: String, token: String) -> Self {
        ClientInfo{
            username: Arc::new(Mutex::new(username)),
            auth_token: Arc::new(Mutex::new(token)),
            channels: Channels::default(),
        }
    }

    pub fn get_initial_messages(&self) -> Vec<tungstenite::Message> {
        let mut out = Vec::new();
        out.push(tungstenite::Message::Text(String::from("CAP REQ :twitch.tv/commands twitch.tv/tags")));
        let (nick, pass) = self.get_auth_commands();
        out.push(pass);
        out.push(nick);
        if let Some(join) = self.channels.join_message() {
            out.push(join);
        }
        return out;
    }

    fn get_auth_commands(&self) -> (tungstenite::Message, tungstenite::Message) {
        return (
            tungstenite::Message::Text(String::from(format!("NICK {}", self.username.lock().unwrap()))),
            tungstenite::Message::Text(String::from(format!("PASS {}", self.auth_token.lock().unwrap()))),
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct Channels {
    channels: Arc<Mutex<Vec<String>>>,
}

impl Channels {
    pub fn new<S>(channels: S) -> Self
        where S: Into<Self> {
        channels.into()
    }

    pub fn join_channel(&mut self, channel: &str) {
        self.channels.lock().unwrap().push(channel.to_string());
    }

    pub fn leave_channel(&mut self, channel: &str) {
        let pos = self.channels.lock().unwrap().iter().position(|s| s == channel);
        if let Some(channel_pos) = pos {
            self.channels.lock().unwrap().remove(channel_pos);
        }
    }

    pub fn join_message(&self) -> Option<tungstenite::Message> {
        if self.channels.lock().unwrap().is_empty() {
            return None;
        }
        let mut msg = String::from("JOIN ");
        for i in self.channels.lock().unwrap().iter() {
            msg += &format!("#{},", i);
        }
        return Some(tungstenite::Message::text(msg));
    }
}

impl From<&str> for Channels {
    fn from(value: &str) -> Self {
        Self { channels: Arc::new(Mutex::new(vec![value.to_string()])) }
    }
}

impl From<String> for Channels {
    fn from(value: String) -> Self {
        Self { channels: Arc::new(Mutex::new(vec![value])) }
    }
}

impl From<&[String]> for Channels {
    fn from(value: &[String]) -> Self {
        Self { channels: Arc::new(Mutex::new(value.into())) }
    }
}

impl From<&[&str]> for Channels {
    fn from(value: &[&str]) -> Self {
        Self { channels: Arc::new(Mutex::new(value.iter().map(|s| s.to_string()).collect())) }
    }
}

impl From<Vec<String>> for Channels {
    fn from(value: Vec<String>) -> Self {
        Self { channels: Arc::new(Mutex::new(value)) }
    }
}

impl From<Vec<&str>> for Channels {
    fn from(value: Vec<&str>) -> Self {
        Self { channels: Arc::new(Mutex::new(value.iter().map(|s| s.to_string()).collect())) }
    }
}
