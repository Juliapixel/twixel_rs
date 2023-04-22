use log::debug;

use crate::{irc_message::{IRCMessage, IRCMessageFormatter}, connection::{Connection, ClientInfo, Channels}};
use std::{sync::{Arc, Mutex, Condvar}, collections::VecDeque};

pub struct ClientBuilder {
    client_info: ClientInfo,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            client_info: ClientInfo::new(
                String::from("justinfan123"),
                String::from("12345")
            )
        }
    }
}

impl ClientBuilder {
    pub fn new_anonymous() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new(username: &str, token: &str) -> Self {
        Self {
            client_info: ClientInfo::new(
                String::from(username),
                String::from(token)
            )
        }
    }

    pub fn channels<T>(mut self, channels: T) -> Self where
        T: Into<Channels> {
        self.client_info.channels = channels.into();
        self
    }

    pub fn build(self) -> TwitchIRCClient {
        self.into()
    }

    pub fn run(self) -> TwitchIRCClient {
        let mut client = self.build();
        client.run();
        return client;
    }
}

impl From<ClientBuilder> for TwitchIRCClient {
    fn from(value: ClientBuilder) -> Self {
        Self {
            client_info: value.client_info.clone(),
            connection: None,
            outgoing: MessageQueue::new(),
            incoming: MessageQueue::new(),
        }
    }
}

pub struct TwitchIRCClient {
    connection: Option<Connection>,
    client_info: ClientInfo,
    pub outgoing: MessageQueue,
    pub incoming: MessageQueue,
}

impl TwitchIRCClient {
    pub fn is_running(&self) -> bool {
        self.connection.is_some()
    }

    pub fn run(&mut self) {
        self.connection = Some(Connection::new(self.client_info.clone()));

        let mut receiver_connection = self.connection.as_ref().unwrap().clone();
        let mut inc = self.incoming.clone();
        std::thread::spawn(move || {
            loop {
                let message = receiver_connection.read().to_string();
                let irc_msg = IRCMessage::try_from(message.as_str()).unwrap();
                inc.add_message(irc_msg);
            }
        });

        let mut sender_connection = self.connection.as_ref().unwrap().clone();
        let mut outg = self.outgoing.clone();
        std::thread::spawn(move || {
            loop {
                let message = outg.get_blocking();
                sender_connection.send(&message.to_string(IRCMessageFormatter::Client));
            }
        });
    }

    pub fn send_message(&mut self, msg: IRCMessage) {
        self.outgoing.add_message(msg);
    }

    pub fn reply_to_message(&mut self, reply: &str, msg: IRCMessage) {
        let mut reply = IRCMessage::text(reply, &msg.channel.clone().unwrap());
        reply.add_tag("reply-parent-msg-id", msg.tags.get_message_id().unwrap());
        self.outgoing.add_message(reply);
    }

    pub fn receive_message(&mut self) -> IRCMessage {
        self.incoming.get_blocking()
    }
}

#[derive(Clone)]
pub struct MessageQueue {
    queue: Arc<Mutex<VecDeque<IRCMessage>>>,
    condvar: Arc<Condvar>,
    is_empty: Arc<Mutex<bool>>
}

impl MessageQueue {
    pub(crate) fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            condvar: Arc::new(Condvar::new()),
            is_empty: Arc::new(Mutex::new(true))
        }
    }

    pub(crate) fn add_message<T>(&mut self, message: T) where
        T: Into<IRCMessage> {
        let mut incoming = self.queue.lock().unwrap();
        if incoming.len() >= 1000 {
            incoming.pop_front();
            incoming.push_back(message.into());
        } else {
            incoming.push_back(message.into());
        }
        *self.is_empty.lock().unwrap() = false;
        self.condvar.notify_all();
    }

    pub fn get_message(&mut self) -> Option<IRCMessage> {
        let mut queue = self.queue.lock().unwrap();
        let out = queue.pop_back();
        if queue.len() == 0 {
            *self.is_empty.lock().unwrap() = true;
        }
        return out;
    }

    pub fn get_blocking(&mut self) -> IRCMessage {
        let guard = self.condvar.wait_while(self.is_empty.lock().unwrap(), |is_empty| *is_empty).unwrap();
        drop(guard);
        self.get_message().unwrap()
    }
}
