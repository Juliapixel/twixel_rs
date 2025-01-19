use crate::{irc_message::tags::OwnedTag, MessageBuilder};

use super::{util::msg_from_param, PrivMsg};

impl PrivMsg<'_> {
    // TODO: treat repeat message avoiders
    pub fn message_text(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in PrivMsg elisWot");
        msg_from_param(msg_param)
    }

    pub fn sender_login(&self) -> Option<&str> {
        self.get_tag(OwnedTag::Login)
    }

    pub fn sender_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::UserId)
    }

    pub fn channel_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::RoomId)
    }

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

    pub fn reply_to(&self, msg: &str) -> MessageBuilder<'_> {
        let reply_id = self
            .get_tag(OwnedTag::ReplyThreadParentMsgId)
            .or_else(|| self.get_tag(OwnedTag::Id));

        let builder = MessageBuilder::privmsg(self.channel_login(), msg);

        if let Some(reply_id) = reply_id {
            builder.add_tag(OwnedTag::ReplyParentMsgId, reply_id)
        } else {
            builder
        }
    }
}
