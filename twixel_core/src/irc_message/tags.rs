use memchr::memchr_iter;
use thiserror::Error;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "chrono")]
use chrono::{Utc, DateTime};
use std::ops::Range;

macro_rules! raw_tags {
    (
        $(#[$top_comment:meta])*
        $tag:ident, $raw_tag:ident,
        $(
            $(#[$comment:meta])*
            $key:literal = $name:ident
        ),*
    ) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Debug, PartialEq, Eq, Clone)]
        #[non_exhaustive]
        $(#[$top_comment])*
        pub enum $raw_tag {
            $(
                $(#[$comment])*
                $name,
            )+
            Unknown(Range<usize>)
        }

        impl $raw_tag {
            pub fn parse(src: &str, range: Range<usize>) -> Self {
                match &src[range.clone()] {
                    $($key => Self::$name,)*
                    _ => {
                        log::warn!("unknown tag parsed! please notify the developers of this issue: {:?}", &src[range.clone()]);
                        Self::Unknown(range)
                    }
                }
            }

            pub fn to_owned<'a>(&self, src: &'a str) -> $tag {
                match self {
                    $(Self::$name => $tag::$name,)*
                    Self::Unknown(r) => $tag::Unknown(String::from(&src[r.clone()]))
                }
            }

            pub fn to_string<'a>(&self, src: &'a str) -> &'a str {
                match self {
                    $(Self::$name => $key,)*
                    Self::Unknown(r) => &src[r.clone()]
                }
            }
        }

        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[cfg_attr(feature = "serde", serde(into = "String", from = "&str"))]
        #[derive(Debug, PartialEq, Eq, Clone)]
        #[non_exhaustive]
        $(#[$top_comment])*
        pub enum $tag {
            $(
                $(#[$comment])*
                $name,
            )+
            Unknown(String)
        }

        impl From<&str> for $tag {
            fn from(val: &str) -> Self {
                match val {
                    $($key => Self::$name,)*
                    _ => Self::Unknown(String::from(val))
                }
            }
        }

        impl From<$tag> for String {
            fn from(val: $tag) -> String {
                match val {
                    $($tag::$name => String::from($key),)+
                    $tag::Unknown(val) => val
                }
            }
        }

        impl<'a> From<&'a $tag> for &'a str {
            fn from(val: &'a $tag) -> &'a str {
                match val {
                    $($tag::$name => $key,)+
                    $tag::Unknown(val) => &val
                }
            }
        }
    };
}

raw_tags!(
    /// [IRCv3 tags](https://ircv3.net/specs/extensions/message-tags.html), that
    /// use [escape sequences](https://ircv3.net/specs/extensions/message-tags.html#escaping-values)
    /// for invalid characters
    OwnedTag, RawTag,
    /// the ID of the message
    "msg-id" = MsgId,
    "badges" = Badges,
    "badge-info" = BadgeInfo,
    "display-name" = DisplayName,
    "emote-only" = EmoteOnly,
    "emotes" = Emotes,
    "flags" = Flags,
    /// the ID of the user
    "id" = Id,
    /// 1 if user is a moderator, 0 if not
    "mod" = Mod,
    "room-id" = RoomId,
    /// 1 if user is subscribed, 0 if not
    "subscriber" = Subscriber,
    /// timestamp of message, in milliseconds since unix epoch
    "tmi-sent-ts" = TmiSentTs,
    /// 1 if user is turbo, 0 if not
    "turbo" = Turbo,
    "user-id" = UserId,
    "user-type" = UserType,
    "client-nonce" = ClientNonce,
    /// 1 if first message in chat, 0 if not
    "first-msg" = FirstMsg,
    "reply-parent-display-name" = ReplyParentDisplayName,
    "reply-parent-msg-body" = ReplyParentMsgBody,
    "reply-parent-msg-id" = ReplyParentMsgId,
    "reply-parent-user-id" = ReplyParentUserId,
    "reply-parent-user-login" = ReplyParentUserLogin,
    "reply-thread-parent-msg-id" = ReplyThreadParentMsgId,
    "reply-thread-parent-user-login" = ReplyThreadParentUserLogin,
    "reply-thread-parent-display-name" = ReplyThreadParentDisplayName,
    "reply-thread-parent-user-id" = ReplyThreadParentuserId,
    /// value of this tag is the amount of time in minutes that a user has to be
    /// following for
    "followers-only" = FollowersOnly,
    "r9k" = R9K,
    "rituals" = Rituals,
    /// value of this tag is the time in seconds for slow mode
    "slow" = Slow,
    /// 1 if sub only mode is enabled, 0 if not
    "subs-only" = SubsOnly,
    "msg-param-cumulative-months" = MsgParamCumulativeMonths,
    "msg-param-displayName" = MsgParamDisplayName,
    "msg-param-login" = MsgParamLogin,
    "msg-param-months" = MsgParamMonths,
    "msg-param-promo-gift-total" = MsgParamPromoGiftTotal,
    "msg-param-promo-name" = MsgParamPromoName,
    "msg-param-recipient-display-name" = MsgParamRecipientDisplayName,
    "msg-param-recipient-id" = MsgParamRecipientId,
    "msg-param-recipient-user-name" = MsgParamRecipientUserName,
    "msg-param-sender-login" = MsgParamSenderLogin,
    "msg-param-sender-name" = MsgParamSenderName,
    "msg-param-should-share-streak" = MsgParamShouldShareStreak,
    "msg-param-streak-months" = MsgParamStreakMonths,
    "msg-param-sub-plan" = MsgParamSubPlan,
    "msg-param-sub-plan-name" = MsgParamSubPlanName,
    "msg-param-viewerCount" = MsgParamViewerCount,
    "msg-param-ritual-name" = MsgParamRitualName,
    "msg-param-threshold" = MsgParamThreshold,
    "msg-param-gift-months" = MsgParamGiftMonths,
    "msg-param-color" = MsgParamColor,
    /// username of user
    "login" = Login,
    "bits" = Bits,
    "system-msg" = SystemMsg,
    "emote-sets" = EmoteSets,
    "thread-id" = ThreadId,
    "message-id" = MessageId,
    "returning-chatter" = ReturningChatter,
    /// color of user, formated as `#XXXXXX`, in RBG hex
    "color" = Color,
    /// present if user is VIP, value is 1
    "vip" = Vip,
    "target-user-id" = TargetUserId,
    "target-msg-id" = TargetMsgId,
    /// [CLEARCHAT](super::command::IrcCommand::ClearChat) only, duration of timeout applied to user, not present if user was banned
    "ban-duration" = BanDuration,
    "msg-param-multimonth-duration" = MsgParamMultimonthDuration,
    "msg-param-was-gifted" = MsgParamWasGifted,
    "msg-param-multimonth-tenure" = MsgParamMultimonthTenure,
    "sent-ts" = SentTs,
    "msg-param-origin-id" = MsgParamOriginId,
    "msg-param-fun-string" = MsgParamFunString,
    "msg-param-sender-count" = MsgParamSenderCount,
    "msg-param-profileImageURL" = MsgParamProfileImageUrl,
    "msg-param-mass-gift-count" = MsgParamMassGiftCount,
    "msg-param-gift-month-being-redeemed" = MsgParamGiftMonthBeingRedeemed,
    "msg-param-anon-gift" = MsgParamAnonGift,
    "custom-reward-id" = CustomRewardId
);

#[derive(Debug, Error)]
pub enum IRCTagParseError {
    #[error("failed to parse the tag due to invalid structure: {0}")]
    TagStructureParseError(String),
    #[error("failed to parse the tag due to unknown error: {0}")]
    ContentParseFailed(String),
    #[error("tag identifier not recognized: {0}")]
    UnknownIdentifier(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawIrcTags {
    /// first item is the [RawTag] enum and the second is the position of the tag's value
    pub(crate) tags: Vec<(RawTag, Range<usize>)>,
}

impl RawIrcTags {
    /// tries to parse a [RawIrcTags] from the tags part of an IRC message, without
    /// the leading `@` and the trailing space
    #[inline]
    pub fn new(raw: &str, raw_start_idx: usize, raw_end_idx: usize) -> Option<Self> {
        let mut tags = Vec::new();

        // position of last found start of tag
        let mut last_pos: usize = raw_start_idx;
        for i in memchr_iter(b';', raw[raw_start_idx..raw_end_idx].as_bytes()) {
            // position of start of next tag
            let pos = i + raw_start_idx + 1;

            // positon of current parsed tag's divider
            let divider = memchr::memchr(b'=', &raw.as_bytes()[last_pos..pos-1])? + last_pos;
            tags.push((RawTag::parse(&raw, last_pos..divider), divider+1..pos-1));

            last_pos = pos;
        }
        // parsing the last tag
        let divider = memchr::memchr(b'=', &raw.as_bytes()[last_pos..])? + last_pos;
        tags.push((RawTag::parse(&raw, last_pos..divider), divider+1..raw_end_idx));

        return Some(Self { tags });
    }

    pub fn get_value<'a>(&self, src: &'a str, tag: RawTag) -> Option<&'a str> {
        let found = self.tags.iter().find(|t| t.0 == tag )?;
        src.get(found.1.clone())
    }

    pub fn get_color(&self, src: &str) -> Option<[u8; 3]> {
        let char_to_int = |byte: u8| -> Option<u8> {
            char::from_u32((byte as u32) << 24)?.to_digit(16).map(|v| v as u8)
        };

        let val = self.get_value(src, RawTag::Color)?;
        if val.len() == 7 {
            let individuals = val[1..].as_bytes();
            return Some([
                char_to_int(individuals[1])? * 16 + char_to_int(individuals[2])?,
                char_to_int(individuals[3])? * 16 + char_to_int(individuals[4])?,
                char_to_int(individuals[5])? * 16 + char_to_int(individuals[6])?,
            ]);
        } else {
            return None;
        }
    }

    #[cfg(feature = "chrono")]
    pub fn get_timestamp(&self, src: &str) -> Option<DateTime<Utc>> {
        let ts = self.get_value(src, RawTag::TmiSentTs)?.parse::<i64>().ok()?;
        return DateTime::<Utc>::from_timestamp(ts / 1000, 0);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OwnedIrcTags {
    pub(crate) tags: Vec<(OwnedTag, String)>
}

impl OwnedIrcTags {
    pub fn get_value(&self, tag: OwnedTag) -> Option<&str> {
        Some(self.tags.iter().find(|t| t.0 == tag )?.1.as_str())
    }

    pub fn get_color(&self) -> Option<[u8; 3]> {
        let char_to_int = |byte: u8| -> Option<u8> {
            char::from_u32((byte as u32) << 24)?.to_digit(16).map(|v| v as u8)
        };

        let val = self.get_value(OwnedTag::Color)?;
        if val.len() == 7 {
            let individuals = val[1..].as_bytes();
            return Some([
                char_to_int(individuals[1])? * 16 + char_to_int(individuals[2])?,
                char_to_int(individuals[3])? * 16 + char_to_int(individuals[4])?,
                char_to_int(individuals[5])? * 16 + char_to_int(individuals[6])?,
            ]);
        } else {
            return None;
        }
    }

    #[cfg(feature = "chrono")]
    pub fn get_timestamp(&self) -> Option<DateTime<Utc>> {
        let ts = self.get_value(OwnedTag::TmiSentTs)?.parse::<i64>().ok()?;
        return DateTime::<Utc>::from_timestamp(ts / 1000, 0);
    }
}

#[cfg(feature = "serde")]
impl Serialize for OwnedIrcTags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.collect_map(self.tags.iter().map(|t| {
            (
                Into::<&str>::into(&t.0),
                t.1.as_str()
            )
        }))
    }
}

// TODO: move this somewhere more adequate
#[derive(Debug, Default)]
pub struct Badge {
    name: String,
    version: i32,
}

#[derive(Debug)]
pub enum SubTier {
    Prime,
    Tier1,
    Tier2,
    Tier3,
}

impl From<&str> for SubTier {
    fn from(value: &str) -> Self {
        match value {
            "1000" => Self::Tier1,
            "2000" => Self::Tier2,
            "3000" => Self::Tier3,
            "Prime" => Self::Prime,
            _ => Self::Tier1,
        }
    }
}