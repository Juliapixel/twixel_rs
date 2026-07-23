#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use memchr::memchr_iter;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    borrow::Cow,
    range::Range,
};
use thiserror::Error;

use super::iter::BadgeIter;

enum Escape {
    Space,
    Backslash,
    Cr,
    Lf,
    Semicolon,
    Other,
    TrailingSlash,
}

fn find_escape_seq(val: &str) -> Option<(Escape, Range<usize>)> {
    let backslash = val.find("\\")?;
    let Some(next) = val[backslash..].chars().nth(1) else {
        return Some((Escape::TrailingSlash, (backslash..backslash + 1).into()));
    };

    let range = (backslash..(backslash + 1 + next.len_utf8())).into();

    match next {
        's' => Some((Escape::Space, range)),
        '\\' => Some((Escape::Backslash, range)),
        'r' => Some((Escape::Cr, range)),
        'n' => Some((Escape::Lf, range)),
        ':' => Some((Escape::Semicolon, range)),
        _ => Some((Escape::Other, range)),
    }
}

pub(crate) fn unescape_tag_value(val: &str) -> Cow<'_, str> {
    let mut pos = 0;
    let mut out = String::with_capacity(val.len());
    while let Some((esc, range)) = find_escape_seq(&val[pos..]) {
        out.push_str(&val[pos..(pos + range.start)]);
        out.push_str(match esc {
            Escape::Space => " ",
            Escape::Backslash => "\\",
            Escape::Cr => "\r",
            Escape::Lf => "\n",
            Escape::Semicolon => ";",
            Escape::Other => &val[(pos + range.start + 1)..(pos + range.end)],
            Escape::TrailingSlash => "",
        });
        pos += range.end
    }
    if out.is_empty() {
        Cow::Borrowed(val)
    } else {
        out.push_str(&val[pos..]);
        Cow::Owned(out)
    }
}

pub(crate) fn escape_tag_value(val: &str) -> Cow<'_, str> {
    let mut last = 0;
    let mut out = String::new();
    for (idx, escapable) in val.match_indices(['\\', ' ', '\r', '\n', ';']) {
        out.push_str(&val[last..idx]);
        out.push_str(match escapable {
            "\\" => "\\\\",
            " " => "\\s",
            "\r" => "\\r",
            "\n" => "\\n",
            ";" => "\\:",
            _ => unreachable!(),
        });
        last = idx + 1;
    }
    if out.is_empty() {
        Cow::Borrowed(val)
    } else {
        out.push_str(&val[last..]);
        Cow::Owned(out)
    }
}

#[cfg(feature = "serde")]
fn ser_range<S>(val: &Range<usize>, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    core::ops::Range::serialize(&(*val).into(), ser)
}

#[cfg(feature = "serde")]
fn deser_range<'de, D>(deser: D) -> Result<Range<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    core::ops::Range::deserialize(deser).map(Into::into)
}

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
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        #[non_exhaustive]
        $(#[$top_comment])*
        pub(crate) enum $raw_tag {
            $(
                #[doc = concat!("the \"", $key, "\" tag")]
                $(#[$comment])*
                $name,
            )+
            /// An unknown tag key value
            #[cfg_attr(feature = "serde", serde(serialize_with = "ser_range", deserialize_with = "deser_range"))]
            Unknown(Range<usize>)
        }

        impl $raw_tag {
            pub fn parse(src: &str, range: Range<usize>) -> Self {
                match &src[range] {
                    $($key => Self::$name,)*
                    _ => {
                        Self::Unknown(range)
                    }
                }
            }

            pub fn to_owned_tag(self, src: &str) -> $tag {
                match self {
                    $(Self::$name => $tag::$name,)*
                    Self::Unknown(r) => $tag::Unknown(String::from(&src[r]))
                }
            }

            pub fn as_str<'a>(&self, src: &'a str) -> &'a str {
                match self {
                    $(Self::$name => $key,)*
                    Self::Unknown(r) => &src[*r]
                }
            }
        }

        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[cfg_attr(feature = "serde", serde(into = "String", from = "&str"))]
        #[derive(Debug, PartialEq, Eq, Clone, Hash)]
        #[non_exhaustive]
        $(#[$top_comment])*
        pub enum $tag {
            $(
                #[doc = concat!("the \"", $key, "\" tag")]
                $(#[$comment])*
                $name,
            )+
            /// An unknown tag key value
            Unknown(String)
        }

        impl $tag {
            /// Panics if trying to convert from Unknown variant
            pub(crate) fn to_raw(&self) -> $raw_tag {
                match self {
                    $(Self::$name => $raw_tag::$name,)+
                    Self::Unknown(_) => panic!(concat!("Cannot convert from ", stringify!($tag::Unknown), " to ", stringify!($raw_tag)))
                }
            }
        }

        impl From<&str> for $tag {
            fn from(val: &str) -> Self {
                match val {
                    $($key => Self::$name,)*
                    _ => Self::Unknown(String::from(val))
                }
            }
        }

        impl ::std::fmt::Display for $tag {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $($tag::$name => f.write_str($key),)+
                    $tag::Unknown(val) => f.write_str(val)
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
    /// the kind of message, not to be confused with Id
    "msg-id" = MsgId,
    /// badges of the message
    "badges" = Badges,
    /// badges of the message in the source channel, only used when shared chat is enabled
    "source-badges" = SourceBadges,
    /// info for the badges of the message
    "badge-info" = BadgeInfo,
    /// info for the badges of the message in the source channel, only used when shared chat is enabled
    "source-badge-info" = SourceBadgeInfo,
    "display-name" = DisplayName,
    "emote-only" = EmoteOnly,
    /// comma-delimited list of emotes in the form `<emote ID>:<start position>-<end position>`
    "emotes" = Emotes,
    "flags" = Flags,
    /// the ID of the message
    "id" = Id,
    /// the ID of the message sent on the source channel, only used when shared chat is enabled
    "source-id" = SourceId,
    /// 1 if user is a moderator, 0 if not
    "mod" = Mod,
    /// the ID of the channel the message was sent in
    "room-id" = RoomId,
    /// the ID of the source channel of the message, only used when shared chat is enabled
    "source-room-id" = SourceRoomId,
    /// 1 if user is subscribed, 0 if not
    "subscriber" = Subscriber,
    /// timestamp of message, in milliseconds since unix epoch
    "tmi-sent-ts" = TmiSentTs,
    /// 1 if user is turbo, 0 if not
    "turbo" = Turbo,
    /// the ID of the user
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
    "msg-param-community-gift-id" = MsgParamCommunityGiftId,
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

/// Error enum for erros when parsing tags
#[derive(Debug, Error)]
pub enum IRCTagParseError {
    /// The structure of the tags did not match what was expected
    #[error("failed to parse the tag due to invalid structure: {0}")]
    TagStructureParseError(String),
    /// Unknown error
    #[error("failed to parse the tag due to unknown error: {0}")]
    ContentParseFailed(String),
    /// Tag identifier was not a known value
    #[error("tag identifier not recognized: {0}")]
    UnknownIdentifier(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawIrcTags {
    /// first item is the [RawTag] enum and the second is the position of the tag's value
    pub(crate) tags: HashMap<RawTag, Range<usize>>,
    pub(crate) unknown: SmallVec<[(Range<usize>, Range<usize>); 4]>,
}

impl RawIrcTags {
    /// tries to parse a [RawIrcTags] from the tags part of an IRC message, without
    /// the leading `@` and the trailing space
    pub(crate) fn new(raw: &str, raw_start_idx: usize, raw_end_idx: usize) -> Option<Self> {
        let mut tags = HashMap::new();
        let mut unknown = SmallVec::new();

        let mut insert_tag =
            |start: Range<usize>, end: Range<usize>| match RawTag::parse(raw, start) {
                RawTag::Unknown(range) => {
                    let key = &raw[range];
                    match unknown.binary_search_by_key(&key, |(k, _)| &raw[*k]) {
                        Ok(found) => unknown[found] = (range, end),
                        Err(idx) => unknown.insert(idx, (range, end)),
                    }
                }
                t => {
                    tags.insert(t, end);
                }
            };

        // position of last found start of tag
        let mut last_pos: usize = raw_start_idx;
        for i in memchr_iter(b';', raw[raw_start_idx..raw_end_idx].as_bytes()) {
            // position of start of next tag
            let pos = i + raw_start_idx + 1;

            // positon of current parsed tag's divider
            if let Some(divider) = memchr::memchr(b'=', &raw.as_bytes()[last_pos..pos - 1]) {
                let divider = divider + last_pos;
                insert_tag((last_pos..divider).into(), (divider + 1..pos - 1).into());
            } else {
                insert_tag((last_pos..pos - 1).into(), (pos - 1..pos - 1).into());
            }

            last_pos = pos;
        }

        // parsing the last tag
        if let Some(divider) =
            memchr::memchr(b'=', &raw.as_bytes()[last_pos..]).map(|d| d + last_pos)
        {
            insert_tag(
                (last_pos..divider).into(),
                (divider + 1..raw_end_idx).into(),
            )
        } else {
            insert_tag(
                (last_pos..raw_end_idx).into(),
                (raw_end_idx..raw_end_idx).into(),
            );
        }

        Some(Self { tags, unknown })
    }

    pub fn value_eq(&self, src: &str, other: &Self, other_src: &str) -> bool {
        self.tags.len() == other.tags.len()
            && self.tags.iter().all(|(k, v)| {
                other
                    .tags
                    .get(k)
                    .is_some_and(|ov| other_src[*ov] == src[*v])
            })
            && self.unknown.len() == other.unknown.len()
            && self.unknown.iter().all(|(k, v)| {
                other
                    .unknown
                    .binary_search_by_key(&(&src[*k], &src[*v]), |(k, v)| {
                        (&other_src[*k], &other_src[*v])
                    })
                    .is_ok()
            })
    }

    /// Retrieves the value associated with the given tag.
    /// # Returns
    /// - `None` if the tag is not present
    /// - An empty string if the tag is present but no key is present
    /// - The value associated with the tag, with escape sequences removed
    pub fn get_value<'a>(&self, src: &'a str, tag: OwnedTag) -> Option<Cow<'a, str>> {
        let found = if let OwnedTag::Unknown(tag) = tag {
            self.unknown.iter().find(|(k, _)| src[*k] == tag)?.1
        } else {
            *self.tags.get(&tag.to_raw())?
        };
        src.get(found).map(unescape_tag_value)
    }

    /// Retrieves the value associated with the given tag.
    /// # Returns
    /// - `None` if the tag is not present
    /// - An empty string if the tag is present but no key is present
    /// - The value associated with the tag, with escape sequences not removed
    pub fn get_raw_value<'a>(&self, src: &'a str, tag: OwnedTag) -> Option<&'a str> {
        let found = if let OwnedTag::Unknown(tag) = tag {
            self.unknown.iter().find(|(k, _)| src[*k] == tag)?.1
        } else {
            *self.tags.get(&tag.to_raw())?
        };
        src.get(found)
    }

    /// Retrieves the value associated with the given tag.
    /// # Returns
    /// - `None` if the tag is not present
    /// - An empty string if the tag is present but no key is present
    /// - The value associated with the tag, with escape sequences removed
    pub fn get_value_by_str<'a>(&self, src: &'a str, tag: &str) -> Option<Cow<'a, str>> {
        let found = self
            .tags
            .iter()
            .find(|t| t.0.as_str(src) == tag)
            .map(|t| *t.1)
            .or_else(|| {
                self.unknown
                    .iter()
                    .find(|(k, _)| &src[*k] == tag)
                    .map(|t| t.1)
            })?;
        src.get(found).map(unescape_tag_value)
    }

    /// Retrieves the value associated with the given tag.
    /// # Returns
    /// - `None` if the tag is not present
    /// - An empty string if the tag is present but no key is present
    /// - The value associated with the tag, with escape sequences not removed
    pub fn get_raw_value_by_str<'a>(&self, src: &'a str, tag: &str) -> Option<&'a str> {
        let found = self
            .tags
            .iter()
            .find(|t| t.0.as_str(src) == tag)
            .map(|t| *t.1)
            .or_else(|| {
                self.unknown
                    .iter()
                    .find(|(k, _)| &src[*k] == tag)
                    .map(|t| t.1)
            })?;
        src.get(found)
    }

    pub fn iter<'a>(&'a self, src: &'a str) -> TagsIter<'a> {
        TagsIter::new(self, src)
    }

    pub fn badge_iter<'a>(&'a self, src: &'a str) -> BadgeIter<'a> {
        BadgeIter::new(src)
    }

    pub fn get_color(&self, src: &str) -> Option<[u8; 3]> {
        let char_to_int =
            |byte: u8| -> Option<u8> { char::from_u32(byte as u32)?.to_digit(16).map(|v| v as u8) };

        let val = self.get_value(src, OwnedTag::Color)?;
        if val.len() == 7 {
            let individuals = val[1..].as_bytes();
            Some([
                char_to_int(individuals[0])? * 16 + char_to_int(individuals[1])?,
                char_to_int(individuals[2])? * 16 + char_to_int(individuals[3])?,
                char_to_int(individuals[4])? * 16 + char_to_int(individuals[5])?,
            ])
        } else {
            None
        }
    }

    #[cfg(feature = "chrono")]
    pub fn get_timestamp(&self, src: &str) -> Option<DateTime<Utc>> {
        let ts = self
            .get_value(src, OwnedTag::TmiSentTs)?
            .parse::<i64>()
            .ok()?;
        DateTime::<Utc>::from_timestamp(ts / 1000, 0)
    }
}

/// An iterator for every tag in an [IrcMessage](crate::IrcMessage)
#[derive(Debug, Clone)]
pub struct TagsIter<'a> {
    src: &'a str,
    tags: hashbrown::hash_map::Iter<'a, RawTag, Range<usize>>,
    unknown: core::slice::Iter<'a, (Range<usize>, Range<usize>)>,
}

impl<'a> TagsIter<'a> {
    fn new(raw: &'a RawIrcTags, src: &'a str) -> Self {
        Self {
            src,
            tags: raw.tags.iter(),
            unknown: raw.unknown.iter(),
        }
    }
}

impl<'a> Iterator for TagsIter<'a> {
    type Item = (OwnedTag, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.tags
            .next()
            .map(|(rt, range)| (rt.to_owned_tag(self.src), &self.src[*range]))
            .or_else(|| {
                self.unknown
                    .next()
                    .map(|(k, v)| (RawTag::Unknown(*k).to_owned_tag(self.src), &self.src[*v]))
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::irc_message::tags::{OwnedTag, RawIrcTags, escape_tag_value, unescape_tag_value};

    #[test]
    fn value_eq() {
        let source = "buh=321;vip=1;color=#123123";
        let source2 = "buh=123;vip=1;buh=321;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");
        let tags2 =
            RawIrcTags::new(source2, 0, source2.len()).expect("failed to parse tags from string");

        assert!(tags.value_eq(source, &tags2, source2));
    }

    #[test]
    fn parse_normal() {
        let source = "buh=123;vip=1;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("123")
        );
        assert_eq!(tags.get_raw_value(source, OwnedTag::Vip), Some("1"));
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn parse_empty() {
        let source = "buh=;vip=1;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("")
        );
        assert_eq!(tags.get_raw_value(source, OwnedTag::Vip), Some("1"));
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn parse_empty_trailing() {
        let source = "vip=1;color=#123123;buh";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(tags.get_raw_value(source, OwnedTag::Vip), Some("1"));
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("")
        );
    }

    #[test]
    fn parse_empty_no_equals() {
        let source = "buh;vip=1;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        dbg!(tags.iter(source));

        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("")
        );
        assert_eq!(tags.get_raw_value(source, OwnedTag::Vip), Some("1"));
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn parse_multi() {
        let source = "vip=123;vip=321;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(tags.get_raw_value(source, OwnedTag::Vip), Some("321"));
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn parse_multi_unknown() {
        let source = "buh=123;buh=321;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("321")
        );
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn parse_multi_unknown2() {
        let source = "buh=123;buh=321;buh=hub;color=#123123";

        let tags =
            RawIrcTags::new(source, 0, source.len()).expect("failed to parse tags from string");

        assert_eq!(
            tags.get_raw_value(source, OwnedTag::Unknown("buh".into())),
            Some("hub")
        );
        assert_eq!(tags.get_raw_value(source, OwnedTag::Color), Some("#123123"));
    }

    #[test]
    fn unescape_tags() {
        let space = "Hello,\\sworld!";
        assert_eq!(unescape_tag_value(space), "Hello, world!");

        let semicolon = "semi\\:";
        assert_eq!(unescape_tag_value(semicolon), "semi;");

        let backslash = "\\\\/";
        assert_eq!(unescape_tag_value(backslash), "\\/");

        let backslash_s = "\\\\s";
        assert_eq!(unescape_tag_value(backslash_s), "\\s");

        let fake = "\\b";
        assert_eq!(unescape_tag_value(fake), "b");

        let multi = "\\s\\s";
        assert_eq!(unescape_tag_value(multi), "  ");

        let all = "\\\\\\s\\:\\r\\n\\a";
        assert_eq!(unescape_tag_value(all), "\\ ;\r\na");

        let trailing = "test\\";
        assert_eq!(unescape_tag_value(trailing), "test");
    }

    #[test]
    fn escape_tags() {
        let space = "Hello, world!";
        assert_eq!(escape_tag_value(space), "Hello,\\sworld!");

        let semicolon = "semi;";
        assert_eq!(escape_tag_value(semicolon), "semi\\:");

        let backslash = "\\/";
        assert_eq!(escape_tag_value(backslash), "\\\\/");

        let space = " ";
        assert_eq!(escape_tag_value(space), "\\s");

        let fake = "\\b";
        assert_eq!(escape_tag_value(fake), "\\\\b");

        let multi = "  ";
        assert_eq!(escape_tag_value(multi), "\\s\\s");

        let all = "\\ ;\r\n\\a";
        assert_eq!(escape_tag_value(all), "\\\\\\s\\:\\r\\n\\\\a");
    }
}
