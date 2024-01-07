use std::{ops::Range, sync::Arc, fmt::Display, slice::Iter};

#[cfg(feature = "serde")]
use serde::{ser::{SerializeStruct, SerializeStructVariant}, Serialize};

use crate::irc_message::{tags::RawIrcTags, prefix::RawPrefix, error::IrcMessageStructureError};

use super::{command::IrcCommand, error::RawIrcMessageParseError, tags::RawTag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIrcMessage {
    pub(crate) raw: Arc<str>,
    pub(crate) tags: Option<RawIrcTags>,
    pub(crate) prefix: Option<RawPrefix>,
    pub(crate) command: IrcCommand,
    pub(crate) params: Vec<Range<usize>>,
}

impl RawIrcMessage {
    pub fn new(val: &str) -> Result<Self, RawIrcMessageParseError> {
        Self::try_from(val)
    }

    pub fn get_tag(&self, tag: RawTag) -> Option<&str> {
        match &self.tags {
            Some(s) => s.get_value(&self.raw, tag),
            None => None,
        }
    }

    pub fn get_host(&self) -> Option<&str> {
        match &self.prefix {
            Some(o) => {
                match o {
                    RawPrefix::OnlyHostname { host } => self.raw.get(host.clone()),
                    RawPrefix::Full { nickname: _, username: _, host } => self.raw.get(host.clone()),
                }
            },
            None => None
        }
    }

    pub fn get_nickname(&self) -> Option<&str> {
        match &self.prefix {
            Some(o) => {
                match o {
                    RawPrefix::Full { nickname, username: _, host: _ } => self.raw.get(nickname.clone()),
                    _ => None,
                }
            },
            None => None
        }
    }

    pub fn get_username(&self) -> Option<&str> {
        match &self.prefix {
            Some(o) => {
                match o {
                    RawPrefix::Full { nickname: _, username, host: _ } => self.raw.get(username.clone()),
                    _ => None,
                }
            },
            None => None
        }
    }

    pub fn get_param(&self, idx: usize) -> Option<&str> {
        self.raw.get(self.params.get(idx)?.clone())
    }

    pub fn params<'a>(&'a self) -> Params<'a> {
        Params {
            src: &self.raw,
            iter: self.params.iter(),
        }
    }

    pub fn get_command(&self) -> IrcCommand {
        self.command
    }
}

impl TryFrom<&str> for RawIrcMessage {
    type Error = RawIrcMessageParseError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use RawIrcMessageParseError as E;

        let raw: Arc<str> = value.into();
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
                .ok_or(IrcMessageStructureError::MissingPrefixSeparator)? + pos;
            let out = RawPrefix::parse(&raw, pos+1, prefix_end);
            pos = prefix_end + 1;
            out
        } else {
            None
        };

        // splits the command from its parameters (if present)
        let cmd = match memchr::memchr2(b' ', b'\r', raw[pos..].as_bytes()) {
            Some(s) => {
                let cmd = &raw[pos..pos+s];
                pos = pos + s + 1;
                cmd
            },
            None => return Err(E::NoCommand)
        };

        let command = IrcCommand::try_from(cmd)?;

        let mut params = Vec::new();

        let mut last_param_start = pos;
        for i in memchr::memchr3_iter(b' ', b'\r', b'\n', raw[pos..].as_bytes()) {
            if raw.as_bytes()[last_param_start] == b':' {
                params.push(
                    if let Some(found_end) = memchr::memchr2(b'\r', b'\n', raw[pos..].as_bytes()) {
                        last_param_start..(found_end + pos)
                    } else {
                        last_param_start..raw.len()
                    }
                );
                break;
            } else {
                params.push(last_param_start..pos+i);
            }
            last_param_start = pos + i + 1;
        }

        return Ok(Self {
            raw,
            tags,
            prefix,
            command,
            params
        })
    }
}

impl Display for RawIrcMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.raw);
    }
}

#[cfg(all(feature = "serde", feature = "unstable"))]
impl Serialize for RawIrcMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            struct TagsSer<'a> {
                raw: &'a str,
                tags: &'a [(RawTag, Range<usize>)]
            }

            impl<'a> Serialize for TagsSer<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer {
                        serializer.collect_map(self.tags.iter().map(|t| {
                            (t.0.to_string(&self.raw), &self.raw[t.1.clone()])
                        }))
                }
            }

            struct PrefixSer<'a> {
                raw: &'a str,
                prefix: &'a RawPrefix
            }

            impl<'a> Serialize for PrefixSer<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer {
                        match self.prefix {
                            RawPrefix::OnlyHostname { host } => {
                                let mut state = serializer.serialize_struct_variant("Prefix", 0, "only_hostname", 1)?;
                                state.serialize_field("host", &self.raw[host.clone()])?;
                                state.end()
                            },
                            RawPrefix::Full { nickname, username, host } => {
                                let mut state = serializer.serialize_struct_variant("Prefix", 0, "full", 3)?;
                                state.serialize_field("nickname", &self.raw[nickname.clone()])?;
                                state.serialize_field("username", &self.raw[username.clone()])?;
                                state.serialize_field("host", &self.raw[host.clone()])?;
                                state.end()
                            },
                        }
                }
            }

            struct ParamsSer<'a> {
                raw: &'a str,
                params: &'a [Range<usize>]
            }

            impl<'a> Serialize for ParamsSer<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer {
                    serializer.collect_seq(self.params.iter().map(|r| &self.raw[r.clone()]))
                }
            }

            let mut msg = serializer.serialize_struct("RawIrcMessage", 5)?;

            msg.serialize_field("tags", &self.tags.as_ref().map(|t| {
                TagsSer { raw: &self.raw, tags: &t.tags }
            }))?;

            msg.serialize_field("prefix", &self.prefix.as_ref().map(|p| {
                PrefixSer { raw: &self.raw, prefix: p }
            }))?;

            msg.serialize_field("command", Into::<&str>::into(self.command))?;

            msg.serialize_field("params", &ParamsSer { raw: &self.raw, params: &self.params })?;

            msg.end()
    }
}

pub struct Params<'a> {
    src: &'a str,
    iter: Iter<'a, Range<usize>>
}

impl<'a> Iterator for Params<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        Some(&self.src[self.iter.next()?.clone()])
    }
}
