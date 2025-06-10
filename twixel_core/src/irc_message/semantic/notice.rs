use crate::irc_message::tags::OwnedTag;

use super::{Notice, util::msg_from_param};

impl Notice<'_> {
    pub fn message_text(&self) -> &str {
        let msg_param = self
            .inner
            .get_param(1)
            .expect("no message in Notice elisWot");
        msg_from_param(msg_param)
    }

    pub fn channel_login(&self) -> &str {
        let chan_param = self
            .inner
            .get_param(0)
            .expect("no channel param in Notice elisWot");
        if !chan_param.starts_with('#') {
            panic!("channel param malformed")
        } else {
            chan_param.split_at(1).1
        }
    }

    pub fn target_user_id(&self) -> Option<&str> {
        self.get_tag(OwnedTag::TargetUserId)
    }

    pub fn kind(&self) -> Option<Result<NoticeKind, NoticeParseError>> {
        self.get_tag(OwnedTag::MsgId).map(|t| t.parse())
    }
}

macro_rules! notice {
    (
        $(#[$top_comment:meta])*
        $enum_name:ident, $error_name:ident
        $(
            $(#[$comment:meta])*
            $key:literal = $name:ident
        ),*
    ) => {
        $(#[$top_comment])*
        #[derive(Debug, Clone, Copy)]
        pub enum $enum_name {
            $(
                $(#[$comment])*
                $name
            ),+
        }

        impl ::std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                f.write_str(self.as_str())
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $error_name;

        impl ::std::fmt::Display for $error_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                f.write_str("no such notice kind found")
            }
        }

        impl ::std::error::Error for $error_name {}

        impl $enum_name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$name => $key),*
                }
            }
        }

        impl ::core::str::FromStr for $enum_name {
            type Err = $error_name;

            // Required method
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $key => Ok(Self::$name),
                    )+
                    _ => Err($error_name)
                }
            }
        }
    }
}

notice!(
    /// the kind of NOTICE message this is
    NoticeKind, NoticeParseError
    /// This room is no longer in emote-only mode.
    "emote_only_off" = EmoteOnlyOff,
    /// This room is now in emote-only mode.
    "emote_only_on" = EmoteOnlyOn,
    /// This room is no longer in followers-only mode.
    "followers_off" = FollowersOff,
    /// This room is now in <duration> followers-only mode.
    "followers_on" = FollowersOn,
    /// This room is now in followers-only mode.
    "followers_on_zero" = FollowersOnZero,
    /// You are permanently banned from talking in <channel>.
    "msg_banned" = Banned,
    /// Your message was not sent because it contained too many unprocessable characters. If you believe this is an error, please rephrase and try again.
    "msg_bad_characters" = BadCharacters,
    /// Your message was not sent because your account is not in good standing in this channel.
    "msg_channel_blocked" = ChannelBlocked,
    /// This channel does not exist or has been suspended.
    "msg_channel_suspended" = ChannelSuspended,
    /// Your message was not sent because it is identical to the previous one you sent, less than 30 seconds ago.
    "msg_duplicate" = Duplicate,
    /// This room is in emote-only mode. You can find your currently available emoticons using the smiley in the chat text area.
    "msg_emoteonly" = EmoteOnly,
    /// This room is in <duration> followers-only mode. Follow <channel> to join the community! Note: These msg_followers tags are kickbacks to a user who does not meet the criteria; that is, does not follow or has not followed long enough.
    "msg_followersonly" = FollowersOnly,
    /// This room is in <duration1> followers-only mode. You have been following for <duration2>. Continue following to chat!
    "msg_followersonly_followed" = FollowersOnlyFollowed,
    /// This room is in followers-only mode. Follow <channel> to join the community!
    "msg_followersonly_zero" = FollowersOnlyZero,
    /// This room is in unique-chat mode and the message you attempted to send is not unique.
    "msg_r9k" = R9K,
    /// Your message was not sent because you are sending messages too quickly.
    "msg_ratelimit" = RateLimit,
    /// Hey! Your message is being checked by mods and has not been sent.
    "msg_rejected" = Rejected,
    /// Your message wasn’t posted due to conflicts with the channel’s moderation settings.
    "msg_rejected_mandatory" = RejectedMandatory,
    /// A verified phone number is required to chat in this channel. Please visit https://www.twitch.tv/settings/security to verify your phone number.
    "msg_requires_verified_phone_number" = RequiresVerifiedPhoneNumber,
    /// This room is in slow mode and you are sending messages too quickly. You will be able to talk again in <number> seconds.
    "msg_slowmode" = SlowMode,
    /// This room is in subscribers only mode. To talk, purchase a channel subscription at https://www.twitch.tv/products/<broadcaster login name>/ticket?ref=subscriber_only_mode_chat.
    "msg_subsonly" = SubsOnly,
    /// You don’t have permission to perform that action.
    "msg_suspended" = Suspended,
    /// You are timed out for <number> more seconds.
    "msg_timedout" = TimedOut,
    /// This room requires a verified account to chat. Please verify your account at https://www.twitch.tv/settings/security.
    "msg_verified_email" = VerifiedEmail,
    /// This room is no longer in slow mode.
    "slow_off" = SlowOff,
    /// This room is now in slow mode. You may send messages every <number> seconds.
    "slow_on" = SlowOn,
    /// This room is no longer in subscribers-only mode.
    "subs_off" = SubsOff,
    /// This room is now in subscribers-only mode.
    "subs_on" = SubsOn,
    /// The community has closed channel <channel> due to Terms of Service violations.
    "tos_ban" = TosBan,
    /// Unrecognized command: <command>
    "unrecognized_cmd" = UnrecognizedCmd
);
