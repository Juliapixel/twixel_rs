use crate::irc_message::tags::OwnedTag;

use super::{util::msg_from_param, ClearMsg};

impl ClearMsg<'_> {
    pub fn target_msg_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::TargetMsgId)
    }

    pub fn message_text(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in PrivMsg elisWot");

        msg_from_param(msg_param)
    }

    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::TargetUserId)
    }

    pub fn room_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::RoomId)
    }

    pub fn target_login(&self) -> Option<&str> {
        self.get_tag(OwnedTag::Login)
    }
}
