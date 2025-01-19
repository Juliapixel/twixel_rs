#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use crate::{auth::Auth, irc_message::builder::MessageBuilder};

const MESSAGE_COOLDOWN_STANDARD: Duration = Duration::from_millis(1500);
const MESSAGE_COOLDOWN_PRIVILEGED: Duration = Duration::from_millis(300);

#[derive(Clone, Debug)]
pub struct SelfStatus {
    pub channels: hashbrown::HashMap<String, ChannelInfo>,
    pub last_sent_message: std::time::Instant,
}

impl Default for SelfStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfStatus {
    pub fn new() -> Self {
        Self {
            channels: hashbrown::HashMap::new(),
            last_sent_message: std::time::Instant::now(),
        }
    }

    pub fn join_channel(&mut self, channel: String) {
        self.channels.insert(
            channel.clone(),
            // TODO: decide when we should request the target channel's display name and id
            // or if they should just be passed to this funcion
            ChannelInfo {
                login: channel.into(),
                display_name: todo!(),
                id: todo!(),
                channel_roles: ChannelRoles::default(),
                last_message: std::time::Instant::now(),
            },
        );
    }

    pub fn leave_channel(&mut self, channel: &str) {
        self.channels.remove(channel);
    }

    /// the ``channel`` parameter should be a lowercase string of the channel's
    /// name.
    pub fn get_channel_info(&self, channel: &str) -> Option<&ChannelInfo> {
        self.channels.get(channel)
    }

    /// the ``channel`` parameter should be a lowercase string of the channel's
    /// name.
    pub fn get_channel_info_mut(&mut self, channel: &str) -> Option<&mut ChannelInfo> {
        self.channels.get_mut(channel)
    }

    pub fn get_join_message(&self) -> Option<MessageBuilder> {
        if !self.channels.is_empty() {
            Some(MessageBuilder::join(self.channels.keys()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChannelInfo {
    pub login: Arc<str>,
    pub display_name: Arc<str>,
    pub id: Arc<str>,
    pub channel_roles: ChannelRoles,
    pub last_message: std::time::Instant,
}

impl ChannelInfo {
    pub async fn new(channel: &str) -> Self {
        // todo!("choose how to create a ChannelInfo");
        ChannelInfo {
            login: channel.into(),
            display_name: channel.into(),
            id: "".into(),
            channel_roles: ChannelRoles::empty(),
            last_message: std::time::Instant::now(),
        }
    }

    pub async fn new_from_id(channel: &str) -> Self {
        todo!("choose how to create a ChannelInfo")
    }

    /// changes ``self.last_message`` to the current instant.
    pub fn message_sent_now(&mut self) {
        self.last_message = std::time::Instant::now();
    }

    /// changes ``self.last_message`` to the specified instant.
    pub fn set_last_message_instant(&mut self, instant: std::time::Instant) {
        self.last_message = instant;
    }

    pub fn is_privileged(&self) -> bool {
        self.channel_roles.is_privileged()
    }

    /// returns ``true`` if you cannot send message
    pub fn is_on_cooldown(&self) -> bool {
        if self.is_privileged() {
            self.last_message.elapsed() < MESSAGE_COOLDOWN_PRIVILEGED
        } else {
            self.last_message.elapsed() < MESSAGE_COOLDOWN_STANDARD
        }
    }

    /// returns ``Some(Duration)`` if you're on cooldown, with the duration being how long it will take until you are
    /// allowed to send another message, otherwise returns ``None``.
    pub fn time_until_cooldown_over(&self) -> Option<std::time::Duration> {
        if !self.is_on_cooldown() {
            return None;
        }
        Some(
            if self.is_privileged() {
                MESSAGE_COOLDOWN_PRIVILEGED
            } else {
                MESSAGE_COOLDOWN_STANDARD
            } - self.last_message.elapsed(),
        )
    }
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Clone, Copy, Default, Debug)]
    pub struct ChannelRoles: u8 {
        const Moderator = 1;
        const Vip = 1 << 1;
        const Subscriber = 1 << 2;
        const Broadcaster = 1 << 3;
    }
}

impl ChannelRoles {
    const PRIVILEGED_MASK: ChannelRoles = ChannelRoles::empty()
        .union(ChannelRoles::Moderator)
        .union(ChannelRoles::Vip)
        .union(ChannelRoles::Broadcaster);

    /// whether you have higher chat privileges in IRC
    pub fn is_privileged(&self) -> bool {
        self.intersects(Self::PRIVILEGED_MASK)
    }
}

#[derive(Clone)]
pub struct ClientInfo {
    pub auth: Auth,
    pub self_info: SelfStatus,
}

impl ClientInfo {
    pub fn new(auth: Auth) -> Self {
        ClientInfo {
            auth,
            self_info: SelfStatus::new(),
        }
    }

    pub fn get_initial_messages(&self) -> Vec<MessageBuilder> {
        let mut out = Vec::new();

        let (nick, pass) = self.get_auth_commands();
        out.push(pass);
        out.push(nick);

        let cap_req = MessageBuilder::cap_req();
        out.push(cap_req);

        if let Some(join) = self.self_info.get_join_message() {
            out.push(join);
        }
        out
    }

    fn get_auth_commands(&self) -> (MessageBuilder<'_>, MessageBuilder<'_>) {
        self.auth.into_commands()
    }
}
