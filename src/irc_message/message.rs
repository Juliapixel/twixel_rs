use std::fmt::Display;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use super::{command::IrcCommand, tags::IrcTags, error::IrcMessageParseError, raw::RawIrcMessage};

// FIXME: this is DUMB, change this ASAP, also fix the casing of the name
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IrcMessage {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tags: IrcTags,
    pub nick: Option<String>,
    pub command: IrcCommand,
    pub channel: Option<String>,
    pub message: Option<String>,
}

pub enum IrcMessageFormatter {
    Server,
    Client
}

impl IrcMessage {
    pub fn to_string(self, formatter: IrcMessageFormatter) -> String {
        match formatter {
            IrcMessageFormatter::Server => {
                let mut out = String::new();
                let mut tags: String = self.tags.into();
                let command: String = self.command.into();
                if !tags.is_empty() {
                    tags += " ";
                    out += &tags;
                }
                if let Some(name) = self.nick {
                    out += &format!(":{0}!{0}@{0}.tmi.twitch.tv ", name);
                } else {
                    out += ":tmi.twitch.tv ";
                }
                out += &format!("{} ", command);
                if let Some(channel) = self.channel {
                    out += &format!("#{} ", channel);
                }
                out += ":";
                if let Some(message) = self.message {
                    out += &message;
                }
                return out
            },
            IrcMessageFormatter::Client => {
                let mut out = String::new();
                let mut tags = String::from(self.tags);
                let command: String = self.command.into();
                if !tags.is_empty() {
                    tags += " ";
                    out += &tags;
                }
                out += &format!("{} ", command);
                if let Some(channel) = self.channel {
                    out += &format!("#{} ", channel);
                }
                out += ":";
                if let Some(message) = self.message {
                    out += &message;
                }
                return out
            }
        }
    }

    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.tags.add_single_tag(key, value);
    }

    pub fn get_color(&self) -> Option<[u8; 3]> {
        self.tags.get_color()
    }

    pub fn is_from_mod(&self) -> bool {
        if let Some(value) = self.tags.get_value("mod") {
            return match value {
                "0" => false,
                "1" => true,
                _ => false
            }
        } else {
            return false;
        }
    }

    pub fn text(message: String, channel: String) -> Self {
        Self {
            command: IrcCommand::PrivMsg,
            channel: Some(channel),
            nick: None,
            message: Some(message),
            tags: IrcTags::default(),
        }
    }

    pub fn whisper(message: String, channel: String) -> Self {
        Self {
            tags: IrcTags::default(),
            nick: None,
            command: IrcCommand::Whisper,
            channel: Some(channel),
            message: Some(message),
        }
    }
}

// FIXME: why &str if we're gonna take ownership of it anyway? remove unnecessary
// clones
impl TryFrom<&str> for IrcMessage {
    type Error = IrcMessageParseError;

    fn try_from(msg: &str) -> Result<IrcMessage, IrcMessageParseError> {
        let raw = RawIrcMessage::try_from(msg)?;

        todo!()
    }
}

impl Default for IrcMessage {
    fn default() -> Self {
        Self {
            tags: IrcTags::default(),
            command: IrcCommand::Useless,
            channel: None,
            nick: None,
            message: None,
        }
    }
}

impl Display for IrcMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.channel.is_some() && self.nick.is_some() && self.message.is_some() {
            write!(
                f,
                "#{} {}: {}",
                self.channel.as_ref().unwrap(),
                self.nick.as_ref().unwrap(),
                self.message.as_ref().unwrap()
            )
        } else {
            write!(f, "{}", format!("{:#?}", self))
        }
    }
}
