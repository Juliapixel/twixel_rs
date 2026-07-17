use crate::{MessageBuilder, irc_message::tags::OwnedTag, user::ChannelRoles};

use super::{PrivMsg, util::msg_from_param};

impl PrivMsg {
    // TODO: treat repeat message avoiders
    /// Text of the message, with invisible and special characters removed
    pub fn message_text(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in PrivMsg elisWot");
        msg_from_param(msg_param)
    }

    /// Returns the senders's role in the channel this was sent in, depending on
    /// tags and badges
    pub fn sender_roles(&self) -> ChannelRoles {
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
            ChannelRoles::LeadModerator,
            self.badges().any(|(n, _)| n == "lead_moderator"),
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

    /// Login of the user who sent this PRIVMSG
    pub fn sender_login(&self) -> Option<&str> {
        self.get_username()
    }

    /// ID of the user who sent this PRIVMSG
    pub fn sender_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::UserId)
    }

    /// ID of the chat where this PRIVMSG was sent
    pub fn channel_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::RoomId)
    }

    /// Login of the chat where this PRIVMSG was sent
    pub fn channel_login(&self) -> &str {
        let chan_param = self
            .inner
            .get_param(0)
            .expect("no channel param in PrivMsg elisWot");
        if !chan_param.starts_with('#') {
            panic!("channel param malformed")
        } else {
            chan_param.split_at(1).1
        }
    }

    /// Whether the message is a /me command and should be highlighted/colored
    pub fn is_me(&self) -> bool {
        self.get_param(1)
            .is_some_and(|p| p.starts_with(":\u{0001}ACTION ") && p.ends_with('\u{0001}'))
    }

    /// The message ID to be used in the ReplyParentMsgId tag when replying
    pub fn reply_to_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::ReplyThreadParentMsgId)
            .or_else(|| self.get_tag_raw(OwnedTag::Id))
    }

    /// Make a new [MessageBuilder](crate::MessageBuilder) that is a reply PRIVMSG
    /// to this
    pub fn reply_to(&self, msg: &str) -> MessageBuilder<'_> {
        let reply_id = self.reply_to_id();

        let builder = MessageBuilder::privmsg(self.channel_login(), msg);

        if let Some(reply_id) = reply_id {
            builder.add_tag(OwnedTag::ReplyParentMsgId, reply_id)
        } else {
            builder
        }
    }
}
