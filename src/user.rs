use std::{sync::Arc, fmt::Write};

use tokio_tungstenite::tungstenite::Message;

const MESSAGE_COOLDOWN_STANDARD_MILLIS: u64 = 1500;
const MESSAGE_COOLDOWN_PRIVILEGED_MILLIS: u64 = 300;

#[derive(Clone, Debug)]
pub struct SelfStatus {
    channels: hashbrown::HashMap<String, ChannelInfo>,
    last_sent_message: std::time::Instant,
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
            channel,
            ChannelInfo {
                display_name: todo!(),
                id: todo!(),
                channel_roles: ChannelRoles::default(),
                last_message: std::time::Instant::now(),
            }
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

    pub fn get_join_message(&self) -> Option<Message> {
        if !self.channels.is_empty() {
            let mut out = String::from("JOIN ");
            for i in self.channels.keys() {
                write!(&mut out, "#{}, ", i).unwrap();
            }
            return Some(Message::Text(out));
        } else {
            return None;
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChannelInfo {
    pub display_name: Arc<str>,
    pub id: Arc<str>,
    pub channel_roles: ChannelRoles,
    pub last_message: std::time::Instant,
}

impl ChannelInfo {
    /// changes ``self.last_message`` to the current instant.
    pub fn message_sent_now(&mut self) {
        self.last_message = std::time::Instant::now();
    }

    /// changes ``self.last_message`` to the specified instant.
    pub fn set_last_message_instant(&mut self, instant: std::time::Instant) {
        self.last_message = instant;
    }

    pub fn is_privileged(&self) -> bool {
        self.channel_roles.is_moderator || self.channel_roles.is_vip
    }

    /// returns ``true`` if you cannot send message
    pub fn is_on_cooldown(&self) -> bool {
        if self.is_privileged() {
            self.last_message.elapsed().as_millis() < MESSAGE_COOLDOWN_PRIVILEGED_MILLIS as u128
        } else {
            self.last_message.elapsed().as_millis() < MESSAGE_COOLDOWN_STANDARD_MILLIS as u128
        }
    }

    /// returns ``Some(Duration)`` if you're on cooldown, with the duration being how long it will take until you are
    /// allowed to send another message, otherwise returns ``None``.
    pub fn time_until_cooldown_over(&self) -> Option<std::time::Duration> {
        if !self.is_on_cooldown() { return None }
        return Some(
            std::time::Duration::from_millis(
                if self.is_privileged() { MESSAGE_COOLDOWN_PRIVILEGED_MILLIS } else { MESSAGE_COOLDOWN_STANDARD_MILLIS }
            ) - self.last_message.elapsed()
        );
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ChannelRoles {
    pub is_moderator: bool,
    pub is_vip: bool,
    pub is_sub: bool,
}
