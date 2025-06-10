use std::{borrow::Cow, fmt::Display, ops::Range, slice::Iter, str::FromStr};

#[cfg(feature = "serde")]
use serde::{
    Serialize,
    ser::{SerializeStruct, SerializeStructVariant},
};
use smallvec::SmallVec;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::irc_message::{error::IrcMessageStructureError, prefix::RawPrefix, tags::RawIrcTags};

use super::{
    ToIrcMessage, command::IrcCommand, error::IrcMessageParseError, iter::IrcMessageParseIter,
    tags::OwnedTag,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrcMessage<'a> {
    raw: Cow<'a, str>,
    tags: Option<RawIrcTags>,
    prefix: Option<RawPrefix>,
    command: IrcCommand,
    params: SmallVec<[Range<usize>; 3]>,
}

impl<'a> IrcMessage<'a> {
    pub fn new(val: Cow<'a, str>) -> Result<Self, IrcMessageParseError> {
        Self::try_from(val)
    }

    pub(crate) fn from_ws_message(ws_message: &'a WsMessage) -> IrcMessageParseIter<'a> {
        let text = ws_message.to_text().unwrap_or_default();

        IrcMessageParseIter::new(text)
    }

    pub fn to_owned(self) -> IrcMessage<'static> {
        IrcMessage::<'static> {
            raw: Cow::Owned(self.raw.into_owned()),
            tags: self.tags,
            prefix: self.prefix,
            command: self.command,
            params: self.params,
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn into_inner(self) -> Cow<'a, str> {
        self.raw
    }

    pub fn badges(&'a self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.tags
            .as_ref()
            .and_then(|t| t.get_value(&self.raw, OwnedTag::Badges).map(|s| (t, s)))
            .map(|(t, src)| t.badge_iter(src))
            .into_iter()
            .flatten()
    }

    pub fn get_tag(&self, tag: OwnedTag) -> Option<&str> {
        match &self.tags {
            Some(s) => s.get_value(&self.raw, tag),
            None => None,
        }
    }

    pub fn tags(&self) -> impl Iterator<Item = (OwnedTag, &str)> {
        self.tags
            .as_ref()
            .map(|t| t.iter(self.raw()))
            .into_iter()
            .flatten()
    }

    #[cfg(feature = "chrono")]
    pub fn get_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.tags.as_ref().and_then(|t| t.get_timestamp(&self.raw))
    }

    pub fn get_color(&self) -> Option<[u8; 3]> {
        self.tags.as_ref().and_then(|t| t.get_color(&self.raw))
    }

    pub fn get_host(&self) -> Option<&str> {
        match &self.prefix {
            Some(o) => match o {
                RawPrefix::OnlyHostname { host } => self.raw.get(host.clone()),
                RawPrefix::Full {
                    nickname: _,
                    username: _,
                    host,
                } => self.raw.get(host.clone()),
            },
            None => None,
        }
    }

    pub fn get_nickname(&self) -> Option<&str> {
        match &self.prefix {
            Some(RawPrefix::Full {
                nickname,
                username: _,
                host: _,
            }) => self.raw.get(nickname.clone()),
            _ => None,
        }
    }

    pub fn get_username(&self) -> Option<&str> {
        match &self.prefix {
            Some(RawPrefix::Full {
                nickname: _,
                username,
                host: _,
            }) => self.raw.get(username.clone()),
            _ => None,
        }
    }

    pub fn get_param(&self, idx: usize) -> Option<&str> {
        self.raw.get(self.params.get(idx)?.clone())
    }

    pub fn params(&self) -> Params<'_> {
        Params {
            src: &self.raw,
            iter: self.params.iter(),
        }
    }

    pub fn get_command(&self) -> IrcCommand {
        self.command
    }
}

impl FromStr for IrcMessage<'static> {
    type Err = IrcMessageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl<'a> TryFrom<Cow<'a, str>> for IrcMessage<'a> {
    type Error = IrcMessageParseError;

    #[inline]
    fn try_from(value: Cow<'a, str>) -> Result<Self, Self::Error> {
        use IrcMessageParseError as E;

        let raw = value;
        let mut pos: usize = 0;

        // parses tags if there are any, and sets `pos` to right
        // right after the trailing space after the tags
        let tags = if raw.starts_with('@') {
            let tag_end = memchr::memchr(b' ', raw[..].as_bytes())
                .ok_or(IrcMessageStructureError::MissingTagSeparator)?;
            let tags = RawIrcTags::new(&raw, 1, tag_end);
            pos = tag_end + 1;
            tags
        } else {
            None
        };

        // parses the prefix, if there is one and then sets `pos`
        // to the first character of the command
        let prefix = if raw[pos..].starts_with(':') {
            let prefix_end = memchr::memchr(b' ', &raw.as_bytes()[pos..])
                .ok_or(IrcMessageStructureError::MissingPrefixSeparator)?
                + pos;
            let out = RawPrefix::parse(&raw, pos + 1, prefix_end);
            pos = prefix_end + 1;
            out
        } else {
            None
        };

        // splits the command from its parameters (if present)
        let cmd = match memchr::memchr2(b' ', b'\r', raw[pos..].as_bytes()) {
            Some(s) => {
                let cmd = &raw[pos..pos + s];
                pos = pos + s + 1;
                cmd
            }
            None => return Err(E::NoCommand),
        };

        let command = IrcCommand::try_from(cmd)?;

        let mut params = SmallVec::new();

        let mut last_param_start = pos;
        for i in memchr::memchr3_iter(b' ', b'\r', b'\n', raw[pos..].as_bytes()) {
            if raw.as_bytes()[last_param_start] == b':' {
                params.push(
                    if let Some(found_end) = memchr::memchr2(b'\r', b'\n', raw[pos..].as_bytes()) {
                        last_param_start..(found_end + pos)
                    } else {
                        last_param_start..raw.len()
                    },
                );
                break;
            } else {
                params.push(last_param_start..pos + i);
            }
            last_param_start = pos + i + 1;
        }

        Ok(Self {
            raw,
            tags,
            prefix,
            command,
            params,
        })
    }
}

impl<'a> TryFrom<&'a str> for IrcMessage<'a> {
    type Error = IrcMessageParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Borrowed(value))
    }
}

impl TryFrom<String> for IrcMessage<'static> {
    type Error = IrcMessageParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Owned(value))
    }
}

impl Display for IrcMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &*self.raw)
    }
}

impl ToIrcMessage for IrcMessage<'_> {
    fn to_message(self) -> String {
        match self.raw {
            Cow::Borrowed(b) => b.to_string(),
            Cow::Owned(o) => o,
        }
    }

    fn get_command(&self) -> IrcCommand {
        self.command
    }
}

#[cfg(all(feature = "serde", feature = "unstable"))]
impl Serialize for IrcMessage<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        struct TagsSer<'a> {
            raw: &'a str,
            tags: &'a RawIrcTags,
        }

        impl Serialize for TagsSer<'_> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_map(self.tags.iter(self.raw))
            }
        }

        struct PrefixSer<'a> {
            raw: &'a str,
            prefix: &'a RawPrefix,
        }

        impl Serialize for PrefixSer<'_> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self.prefix {
                    RawPrefix::OnlyHostname { host } => {
                        let mut state =
                            serializer.serialize_struct_variant("Prefix", 0, "OnlyHostname", 1)?;
                        state.serialize_field("host", &self.raw[host.clone()])?;
                        state.end()
                    }
                    RawPrefix::Full {
                        nickname,
                        username,
                        host,
                    } => {
                        let mut state =
                            serializer.serialize_struct_variant("Prefix", 0, "Full", 3)?;
                        state.serialize_field("nickname", &self.raw[nickname.clone()])?;
                        state.serialize_field("username", &self.raw[username.clone()])?;
                        state.serialize_field("host", &self.raw[host.clone()])?;
                        state.end()
                    }
                }
            }
        }

        struct ParamsSer<'a> {
            raw: &'a str,
            params: &'a [Range<usize>],
        }

        impl Serialize for ParamsSer<'_> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_seq(self.params.iter().map(|r| &self.raw[r.clone()]))
            }
        }

        let mut msg = serializer.serialize_struct("IrcMessage", 5)?;

        msg.serialize_field(
            "tags",
            &self.tags.as_ref().map(|t| TagsSer {
                raw: &self.raw,
                tags: t,
            }),
        )?;

        msg.serialize_field(
            "prefix",
            &self.prefix.as_ref().map(|p| PrefixSer {
                raw: &self.raw,
                prefix: p,
            }),
        )?;

        msg.serialize_field("command", Into::<&str>::into(self.command))?;

        msg.serialize_field(
            "params",
            &ParamsSer {
                raw: &self.raw,
                params: &self.params,
            },
        )?;

        msg.end()
    }
}

pub struct Params<'a> {
    src: &'a str,
    iter: Iter<'a, Range<usize>>,
}

impl<'a> Iterator for Params<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        Some(&self.src[self.iter.next()?.clone()])
    }
}

#[test]
fn from_ws_message() {
    const MSGS: &str = "@badge-info=;badges=moments/2;client-nonce=9297a96d510091fa87c81eaa9e5bb8e3;color=#E4E5FF;display-name=MELLOWFLEUR;emotes=;first-msg=0;flags=;id=1ada6902-aafe-452a-8651-1fe711ddd7d1;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1680318910689;turbo=0;user-id=45179149;user-type= :mellowfleur!mellowfleur@mellowfleur.tmi.twitch.tv PRIVMSG #xqc :yes\r
@badge-info=;badges=moments/2;client-nonce=da0ef47ebddf148067c685599dd6bc90;color=#8A2BE2;display-name=lonelythomas;emotes=;first-msg=0;flags=;id=91c3b354-95b7-4509-a337-3b86c194b141;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1680318910693;turbo=0;user-id=217061103;user-type= :lonelythomas!lonelythomas@lonelythomas.tmi.twitch.tv PRIVMSG #xqc :LETHIMCOOK\r
@badge-info=subscriber/19;badges=subscriber/18,bits/100;client-nonce=b937ab21b00c4f01bd6b729e9b47b665;color=#FFFFFF;display-name=ink6h;emotes=;first-msg=0;flags=;id=5364e52d-baa5-42fa-95a5-d719e17e41dd;mod=0;returning-chatter=0;room-id=71092938;subscriber=1;tmi-sent-ts=1680318911064;turbo=0;user-id=168511883;user-type= :ink6h!ink6h@ink6h.tmi.twitch.tv PRIVMSG #xqc :ye\r
@badge-info=;badges=;color=;display-name=getoutofmyhead123;emote-only=1;emotes=emotesv2_04dd118ef04a49c1aa0caa7fc3144369:0-4,6-10,12-16;first-msg=0;flags=;id=225dcdf8-c734-4f62-bb30-af49f2af32e9;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1680318911099;turbo=0;user-id=880902531;user-type= :getoutofmyhead123!getoutofmyhead123@getoutofmyhead123.tmi.twitch.tv PRIVMSG #xqc :xqcLL xqcLL xqcLL\r";
    let msg = WsMessage::Text(MSGS.into());
    for msg in IrcMessage::from_ws_message(&msg) {
        assert!(msg.is_ok(), "{:?}", msg);
    }
}
