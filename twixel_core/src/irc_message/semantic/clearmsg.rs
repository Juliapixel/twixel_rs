use crate::irc_message::tags::OwnedTag;

use super::{ClearMsg, util::msg_from_param};

impl ClearMsg {
    /// ID of the message that was deleted
    pub fn target_msg_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::TargetMsgId)
    }

    /// Text of the deleted message
    pub fn message_text(&self) -> Option<&str> {
        self
            .inner
            .get_param(1)
            .map(msg_from_param)
    }

    /// ID of the user whose message was deleted
    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::TargetUserId)
    }

    /// ID of the channel where the message was deleted
    pub fn room_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::RoomId)
    }

    /// Login of the channel where the message was deleted
    pub fn channel_login(&self) -> Option<&str> {
        self.get_param(0).and_then(|p| p.split_at_checked(1).map(|s| s.1))
    }
}
