use std::{convert::Infallible, fmt::Display, ops::Range, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawPrefix {
    OnlyHostname {
        host: Range<usize>,
    },
    Full {
        nickname: Range<usize>,
        username: Range<usize>,
        host: Range<usize>,
    },
}

impl RawPrefix {
    #[inline]
    pub fn parse(raw: &str, prefix_start: usize, prefix_end: usize) -> Option<Self> {
        match memchr::memchr(b'!', raw[prefix_start..prefix_end].as_bytes()) {
            Some(user_separator) => {
                let user_separator_pos = user_separator + prefix_start;
                let host_separator_pos =
                    memchr::memchr(b'@', raw[user_separator_pos..prefix_end].as_bytes())?
                        + user_separator_pos;

                Some(Self::Full {
                    nickname: prefix_start..user_separator_pos,
                    username: user_separator_pos + 1..host_separator_pos,
                    host: host_separator_pos + 1..prefix_end,
                })
            }
            None => Some(Self::OnlyHostname {
                host: prefix_start..prefix_end,
            }),
        }
    }
}

/// The "prefix" part of the IRC message
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedPrefix {
    /// A prefix which only specifies the hostname
    OnlyHostname {
        /// The hostname
        host: String,
    },
    /// A full prefix
    Full {
        /// The nickname segment
        nickname: String,
        /// The username segment
        username: String,
        /// The hostname
        host: String,
    },
}

impl FromStr for OwnedPrefix {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value
            .split_once('@')
            .and_then(|(l, r)| Some((l.split_once('!')?, r)))
        {
            Some(((nickname, username), host)) => Ok(Self::Full {
                nickname: nickname.into(),
                username: username.into(),
                host: host.into(),
            }),
            None => Ok(Self::OnlyHostname { host: value.into() }),
        }
    }
}

impl From<&str> for OwnedPrefix {
    #[inline]
    fn from(value: &str) -> Self {
        value.parse().unwrap()
    }
}

impl Display for OwnedPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnedPrefix::OnlyHostname { host } => write!(f, ":{host}"),
            OwnedPrefix::Full {
                nickname,
                username,
                host,
            } => write!(f, ":{nickname}!{username}@{host}"),
        }
    }
}
