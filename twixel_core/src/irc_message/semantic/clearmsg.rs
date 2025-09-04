use crate::irc_message::tags::OwnedTag;

use super::{ClearMsg, util::msg_from_param};

impl ClearMsg {
    pub fn target_msg_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::TargetMsgId)
    }

    pub fn message_text(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in PrivMsg elisWot");

        msg_from_param(msg_param)
    }

    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::TargetUserId)
    }

    pub fn room_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::RoomId)
    }

    pub fn target_login(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::Login)
    }
}
