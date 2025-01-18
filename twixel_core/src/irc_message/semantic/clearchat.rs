use crate::irc_message::tags::OwnedTag;

use super::{util::msg_from_param, ClearChat};

pub enum TimeoutDuration {
    Permanent,
    Temporary(std::time::Duration),
}

impl ClearChat<'_> {
    pub fn target_msg_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::TargetMsgId)
    }

    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::TargetUserId)
    }

    pub fn room_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::RoomId)
    }

    pub fn duration(&self) -> TimeoutDuration {
        match self.get_tag(OwnedTag::BanDuration) {
            Some(dur) => {
                TimeoutDuration::Temporary(std::time::Duration::from_secs(dur.parse().unwrap()))
            }
            None => TimeoutDuration::Permanent,
        }
    }

    pub fn target_login(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in PrivMsg elisWot");

        msg_from_param(msg_param)
    }
}
