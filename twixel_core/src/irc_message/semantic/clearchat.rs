use crate::irc_message::tags::OwnedTag;

use super::ClearChat;

/// Duration of the timeout/ban
pub enum TimeoutDuration {
    /// Permanent ban
    Permanent,
    /// Temporary timeout, specified in seconds
    Temporary(std::time::Duration),
}

impl ClearChat {
    /// User ID of the target of the timeout/ban
    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::TargetUserId)
    }

    /// ID of the channel the timeout/ban occurred in
    pub fn room_id(&self) -> Option<&str> {
        self.get_tag_raw(OwnedTag::RoomId)
    }

    /// Duration of time timeout/ban
    pub fn duration(&self) -> TimeoutDuration {
        match self.get_tag(OwnedTag::BanDuration) {
            Some(dur) if dur.is_empty() => {
                TimeoutDuration::Permanent
            }
            Some(dur) => {
                TimeoutDuration::Temporary(std::time::Duration::from_secs(dur.parse().unwrap()))
            }
            None => TimeoutDuration::Permanent,
        }
    }

    /// Login of the target of the timeout/ban
    pub fn channel_login(&self) -> Option<&str> {
        self.get_param(0).and_then(|p| p.split_at_checked(1).map(|s| s.1))
    }

    /// Login of the channel the timeout/ban occurred in
    pub fn target_login(&self) -> Option<&str> {
        self.get_param(1).and_then(|p| p.split_at_checked(1).map(|s| s.1))
    }
}
