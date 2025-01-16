use crate::{auth::Auth, connection::{error::ConnectionError, Connection}, irc_message::{builder::MessageBuilder, message::IrcMessage, tags::OwnedTag, ToIrcMessage}, user::ClientInfo};
use std::{sync::{Arc, Mutex, Condvar}, collections::VecDeque};
use log::debug;
use rand::Rng;

pub struct ClientBuilder {
    client_info: ClientInfo,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let mut username = String::from("justinfan");
        let random = rng.gen_range(1..1_000_000);
        username += random.to_string().as_str();
        let pass = rng.gen_range(1..1_000_000).to_string();
        debug!("new anonymous login created: {} {}", &username, &pass);
        Self {
            client_info: ClientInfo::new(Auth::default())
        }
    }
}

impl ClientBuilder {
    pub fn new_anonymous() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new(username: String, token: String) -> Self {
        Self {
            client_info: ClientInfo::new(
                Auth::OAuth { username, token }
            )
        }
    }

    pub fn channels<T>(mut self, channels: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<String>
    {
        for i in channels {
            self.client_info.self_info.join_channel(i.into());
        }
        self
    }

    pub fn build(self) -> TwitchIrcClient {
        self.into()
    }

    pub async fn run(self) -> TwitchIrcClient {
        let mut client = self.build();
        client.run().await;
        return client;
    }
}

impl From<ClientBuilder> for TwitchIrcClient {
    fn from(value: ClientBuilder) -> Self {
        Self {
            client_info: value.client_info.clone(),
            connection: None,
        }
    }
}

pub struct TwitchIrcClient {
    connection: Option<Connection>,
    client_info: ClientInfo,
}

impl TwitchIrcClient {
    pub fn is_running(&self) -> bool {
        self.connection.is_some()
    }

    pub async fn run(&mut self) {
        self.connection = Some(Connection::new(self.client_info.clone()).await);
        self.connection.as_mut().unwrap().start().await;
    }

    pub async fn send_message(&mut self, msg: impl ToIrcMessage) -> Result<(), ConnectionError> {
        if let Some(conn) = &mut self.connection {
            conn.send(msg).await;
        }
    }

    pub async fn receive_message(&mut self) -> Result<IrcMessage<'static>, ConnectionError> {
        if let Some(conn) = &mut self.connection {
            conn.receive().await?
        } else {
            panic!("can't receive messages before calling run() on TwitchIrcClient!");
        }
    }

    pub async fn reply_to_message(&mut self, reply: &str, msg: IrcMessage<'_>) -> Result<(), ConnectionError> {
        if let Some(reply_id) = msg.get_tag(OwnedTag::Id) {
            let reply = MessageBuilder::privmsg(msg.get_param(0).unwrap(), &reply)
                .add_tag(OwnedTag::ReplyParentMsgId, reply_id);

            self.send_message(reply).await?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct MessageQueue {
    queue: Arc<Mutex<VecDeque<IrcMessage>>>,
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
        T: Into<IrcMessage> {
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

    pub fn get_message(&mut self) -> Option<IrcMessage> {
        let mut queue = self.queue.lock().unwrap();
        let out = queue.pop_back();
        if queue.len() == 0 {
            *self.is_empty.lock().unwrap() = true;
        }
        return out;
    }

    pub fn get_blocking(&mut self) -> IrcMessage {
        let guard = self.condvar.wait_while(self.is_empty.lock().unwrap(), |is_empty| *is_empty).unwrap();
        drop(guard);
        self.get_message().unwrap()
    }
}
