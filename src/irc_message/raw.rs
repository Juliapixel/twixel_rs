use std::{ops::Range, sync::Arc, fmt::Display};

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

    pub fn get_command(&self) -> IrcCommand {
        self.command
    }
}

impl TryFrom<&str> for RawIrcMessage {
    type Error = RawIrcMessageParseError;

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
                    last_param_start..memchr::memchr2(b'\r', b'\n', raw[pos..].as_bytes())
                    .unwrap_or(raw.len()) + pos
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
        if let Some(tags) = &self.tags {
            write!(f, "@")?;
            let mut first = true;
            for i in &tags.tags {
                write!(f, "{}{}={}", if first { "" } else { ";" }, i.0.to_string(&self.raw), &self.raw[i.1.clone()])?;
                first = false;
            }
            write!(f, " ")?;
        }
        if let Some(prefix) = &self.prefix {
            write!(f, "{} ", prefix.to_string(&self.raw))?;
        }
        write!(f, "{}", Into::<&str>::into(self.command))?;

        for (i, val) in self.params.iter().enumerate() {
            write!(
                f,
                " {}{}",
                &self.raw[val.clone()],
                if i == self.params.len() - 1 { "\r\n" } else { "" }
            )?;
        }

        return Ok(());
    }
}
