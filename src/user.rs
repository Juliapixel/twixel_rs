use std::{sync::Arc, fmt::Write, time::Duration};

use crate::auth::Auth;

// why is this just millis brah
const MESSAGE_COOLDOWN_STANDARD: Duration = Duration::from_millis(1500);
const MESSAGE_COOLDOWN_PRIVILEGED: Duration = Duration::from_millis(300);

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
            // TODO: decide when we should request the target channel's display name and id
            // or if they should just be passed to this funcion
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

    pub fn get_join_message(&self) -> Option<String> {
        if !self.channels.is_empty() {
            let mut out = String::from("JOIN ");
            for i in self.channels.keys() {
                write!(&mut out, "#{}, ", i).unwrap();
            }
            return Some(out);
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
            return self.last_message.elapsed() < MESSAGE_COOLDOWN_PRIVILEGED;
        } else {
            return self.last_message.elapsed() < MESSAGE_COOLDOWN_STANDARD;
        }
    }

    /// returns ``Some(Duration)`` if you're on cooldown, with the duration being how long it will take until you are
    /// allowed to send another message, otherwise returns ``None``.
    pub fn time_until_cooldown_over(&self) -> Option<std::time::Duration> {
        if !self.is_on_cooldown() { return None }
        return Some(
                if self.is_privileged() {
                    MESSAGE_COOLDOWN_PRIVILEGED
                } else {
                    MESSAGE_COOLDOWN_STANDARD
                } - self.last_message.elapsed()
        );
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ChannelRoles {
    pub is_moderator: bool,
    pub is_vip: bool,
    pub is_sub: bool,
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

    pub fn get_initial_messages(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::from("CAP REQ :twitch.tv/commands twitch.tv/tags"));
        let (nick, pass) = self.get_auth_commands();
        out.push(pass);
        out.push(nick);
        if let Some(join) = self.self_info.get_join_message() {
            out.push(join);
        }
        return out;
    }

    fn get_auth_commands(&self) -> (String, String) {
        self.auth.into_commands()
    }
}
