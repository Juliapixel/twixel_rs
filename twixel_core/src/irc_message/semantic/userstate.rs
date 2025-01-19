use crate::{irc_message::tags::OwnedTag, user::ChannelRoles};

use super::UserState;

impl UserState<'_> {
    pub fn channel_login(&self) -> &str {
        self.get_param(0)
            .expect("malformed channel login param")
            .split_at(1)
            .1
    }

    pub fn roles(&self) -> ChannelRoles {
        let mut roles = ChannelRoles::empty();

        roles.set(
            ChannelRoles::Vip,
            self.get_tag(OwnedTag::Vip)
                .map(|t| t == "1")
                .unwrap_or(false),
        );
        roles.set(
            ChannelRoles::Moderator,
            self.get_tag(OwnedTag::Mod)
                .map(|t| t == "1")
                .unwrap_or(false),
        );
        roles.set(
            ChannelRoles::Subscriber,
            self.get_tag(OwnedTag::Subscriber)
                .map(|t| t == "1")
                .unwrap_or(false),
        );
        roles.set(
            ChannelRoles::Broadcaster,
            self.badges().any(|(k, _v)| k == "broadcaster"),
        );

        roles
    }

    pub fn is_mod(&self) -> bool {
        self.get_tag(OwnedTag::Mod).is_some()
    }
}
