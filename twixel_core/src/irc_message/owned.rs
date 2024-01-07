use std::fmt::Display;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{command::IrcCommand, tags::OwnedTag, prefix::OwnedPrefix};

#[cfg_attr(all(feature = "serde", feature = "unstable"), derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct OwnedIrcMessage {
    pub tags: Option<Vec<(OwnedTag, String)>>,
    pub prefix: Option<OwnedPrefix>,
    pub command: IrcCommand,
    pub params: Vec<String>
}

impl OwnedIrcMessage {
    pub fn pong(val: String) -> Self {
        Self {
            tags: None,
            prefix: None,
            command: IrcCommand::Pong,
            params: vec![val],
        }
    }

    pub fn privmsg(mut channel: String, mut message: String) -> Self {
        channel.insert(0, '#');
        message.insert(0, ':');
        Self {
            tags: None,
            prefix: None,
            command: IrcCommand::PrivMsg,
            params: vec![channel, message],
        }
    }
}

impl Display for OwnedIrcMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tags) = &self.tags {
            if !tags.is_empty() {
                write!(f, "@")?;
                let mut first = true;
                for i in tags {
                    write!(f, "{}{}={}", if first { "" } else { ";" }, Into::<&str>::into(&i.0), i.1)?;
                    first = false;
                }
                write!(f, " ")?;
            }
        }
        if let Some(prefix) = &self.prefix {
            write!(f, "{} ", prefix.to_string())?;
        }
        write!(f, "{}", Into::<&str>::into(self.command))?;

        for (i, val) in self.params.iter().enumerate() {
            write!(
                f,
                " {}{}",
                val,
                if i == self.params.len() - 1 { "\r\n" } else { "" }
            )?;
        }

        return Ok(());
    }
}
