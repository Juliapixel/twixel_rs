use std::{ops::Range, fmt::Display};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawPrefix {
    OnlyHostname { host: Range<usize> },
    Full{ nickname: Range<usize>, username: Range<usize>, host: Range<usize> }
}

impl RawPrefix {
    pub fn parse(raw: &str, prefix_start: usize, prefix_end: usize) -> Option<Self> {
        match memchr::memchr(b'!', raw[prefix_start..prefix_end].as_bytes()) {
            Some(user_separator) => {
                let user_separator_pos = user_separator + prefix_start;
                let host_separator_pos =
                    memchr::memchr(b'@', raw[user_separator_pos..prefix_end].as_bytes())? + user_separator_pos;

                Some(Self::Full {
                    nickname: prefix_start..user_separator_pos,
                    username: user_separator_pos+1..host_separator_pos,
                    host: host_separator_pos+1..prefix_end,
                })
            },
            None => Some(Self::OnlyHostname { host: prefix_start..prefix_end }),
        }
    }

    pub fn to_string<'a>(&self, src: &'a str) -> String {
        match self {
            RawPrefix::OnlyHostname { host } => {
                format!(":{}", &src[host.clone()])
            },
            RawPrefix::Full { nickname, username, host } => {
                format!(":{}!{}@{}", &src[nickname.clone()], &src[username.clone()], &src[host.clone()])
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedPrefix {
    OnlyHostname{ host: String },
    Full{ nickname: String, username: String, host: String }
}

impl From<&str> for OwnedPrefix {
    fn from(value: &str) -> Self {
        match value.split_once('@') {
            Some(splits) => {
                let (nickname, username) = splits.0.split_once('!').unwrap();
                let hostname = splits.1.to_string();
                Self::Full {
                    nickname: String::from(nickname),
                    username: String::from(username),
                    host: hostname
                }
            },
            None => Self::OnlyHostname{ host: value.to_string() }
        }
    }
}

impl Display for OwnedPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnedPrefix::OnlyHostname { host } => write!(f, ":{host}"),
            OwnedPrefix::Full { nickname, username, host } => write!(f, ":{nickname}!{username}@{host}"),
        }
    }
}
