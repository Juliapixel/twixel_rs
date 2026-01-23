use std::{borrow::Cow, fmt::Write};

use hashbrown::HashMap;

use crate::irc_message::{PrivMsg, tags::escape_tag_value};

use super::{ToIrcMessage, command::IrcCommand, prefix::OwnedPrefix, tags::OwnedTag};

/// Helper struct to make new IRCv3 messages
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MessageBuilder<'a> {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_tags"))]
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_tags"))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    tags: HashMap<OwnedTag, Cow<'a, str>>,
    prefix: Option<OwnedPrefix>,
    /// The IRCv3 command
    pub command: IrcCommand,
    #[cfg_attr(feature = "serde", serde(borrow))]
    params: Vec<Cow<'a, str>>,
}

/// Error occurred while creating a [MessageBuilder]
#[derive(Debug, thiserror::Error)]
pub enum MessageBuilderError {
    /// The message used to create a [MessageBuilder] was missing a required tag
    #[error("could not create builder from message due to a missing tag")]
    MissingTag,
}

impl std::fmt::Debug for MessageBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageBuilder")
            .field("tags", &self.tags)
            .field("prefix", &self.prefix)
            .field("command", &self.command)
            .field(
                "params",
                if self.command == IrcCommand::Pass {
                    &"[TOKEN REDACTED]"
                } else {
                    &self.params
                },
            )
            .finish()
    }
}

impl<'a> MessageBuilder<'a> {
    /// Make a new [MessageBuilder] with the Given [IrcCommand]
    pub fn new(command: IrcCommand) -> Self {
        Self {
            tags: HashMap::new(),
            prefix: None,
            command,
            params: vec![],
        }
    }

    /// Convenience method to make a new `PRIVMSG` that reponds to another `PRIVMSG`,
    /// using Twitch's `reply-parent-msg-id` tag.
    pub fn reply(msg: &'a PrivMsg, message: &'a str) -> Result<Self, MessageBuilderError> {
        let Some(parent_id) = msg
            .get_tag(OwnedTag::ReplyThreadParentMsgId)
            .or(msg.get_tag(OwnedTag::Id))
        else {
            return Err(MessageBuilderError::MissingTag);
        };
        Ok(
            Self::privmsg(msg.get_param(0).unwrap().split_at(1).1, message)
                .add_tag(OwnedTag::ReplyParentMsgId, parent_id),
        )
    }

    /// Build a finished IRC message in string form
    pub fn build(self) -> String {
        let mut out = String::new();

        // tags
        for (idx, tag) in self.tags.iter().enumerate() {
            if idx == 0 {
                write!(&mut out, "@").unwrap();
            } else {
                write!(&mut out, ";").unwrap();
            }
            write!(&mut out, "{}={}", &tag.0, tag.1).unwrap()
        }

        if !self.tags.is_empty() {
            write!(&mut out, " ").unwrap();
        }

        // prefix
        if let Some(prefix) = &self.prefix {
            write!(&mut out, "{prefix} ").unwrap();
        }

        // command
        write!(&mut out, "{}", self.command).unwrap();

        // params
        for param in self.params.into_iter() {
            write!(&mut out, " {param}",).unwrap();
        }

        // CRLF EOL
        write!(&mut out, "\r\n").unwrap();

        out
    }

    /// Add new tag-value pair
    pub fn add_tag(mut self, tag: OwnedTag, value: impl Into<Cow<'a, str>>) -> Self {
        let value = match value.into() {
            Cow::Borrowed(s) => escape_tag_value(s),
            Cow::Owned(s) => {
                match escape_tag_value(&s) {
                    Cow::Borrowed(_) => Cow::Owned(s),
                    Cow::Owned(s) => Cow::Owned(s),
                }
            },
        };
        self.tags.insert(tag, value);
        self
    }

    /// Add new param
    pub fn add_param(mut self, param: impl Into<Cow<'a, str>>) -> Self {
        self.params.push(param.into());
        self
    }

    /// Set message prefix
    pub fn prefix(mut self, prefix: OwnedPrefix) -> Self {
        let _ = self.prefix.insert(prefix);
        self
    }

    /// Convenience method to make a new `PRIVMSG` message
    pub fn privmsg(channel: &'a str, message: &str) -> Self {
        let chan_param = if channel.starts_with('#') {
            Cow::Borrowed(channel)
        } else {
            Cow::Owned(format!("#{channel}"))
        };
        Self::new(IrcCommand::PrivMsg)
            .add_param(chan_param)
            .add_param(format!(":{message}"))
    }

    /// Convenience method to repond to data from a `PING`
    ///
    /// `data` MUST be the `PING` message's last param
    pub fn pong(data: &'a str) -> Self {
        Self::new(IrcCommand::Pong).add_param(data)
    }

    /// Convenience method to make a new `JOIN` message for many channels
    pub fn join(channels: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        let mut channel_list = String::new();
        for (idx, chan) in channels.into_iter().enumerate() {
            if idx > 0 {
                write!(&mut channel_list, ",").unwrap()
            }
            write!(&mut channel_list, "#{chan}").unwrap()
        }
        Self::new(IrcCommand::Join).add_param(channel_list)
    }

    /// Convenience method to make a new `PART` message for many channels
    pub fn part(channels: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        let mut channel_list = String::new();
        for (idx, chan) in channels.into_iter().enumerate() {
            if idx > 0 {
                write!(&mut channel_list, ",").unwrap()
            }
            write!(&mut channel_list, "#{chan}").unwrap()
        }
        Self::new(IrcCommand::Part).add_param(channel_list)
    }

    /// Convenience method to make a new `CAP REQ` message for Twitch
    pub fn cap_req() -> Self {
        Self::new(IrcCommand::Cap)
            .add_param("REQ")
            .add_param(":twitch.tv/commands twitch.tv/tags")
    }

    /// Convert from a [MessageBuilder] using borrowed data to using owned data
    pub fn to_owned(self) -> MessageBuilder<'static> {
        let mut new = MessageBuilder::<'static>::new(self.command);
        new.params = self
            .params
            .into_iter()
            .map(|p| p.into_owned().into())
            .collect();
        new.tags = self
            .tags
            .into_iter()
            .map(|(t, v)| (t, v.into_owned().into()))
            .collect();
        new.prefix = self.prefix;
        new
    }
}

impl ToIrcMessage for MessageBuilder<'_> {
    fn get_command(&self) -> IrcCommand {
        self.command
    }

    fn to_message(self) -> String {
        self.build()
    }
}

#[cfg(feature = "serde")]
fn serialize_tags<S: serde::Serializer>(
    value: &HashMap<OwnedTag, Cow<'_, str>>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    ser.collect_map(value.iter())
}

#[cfg(feature = "serde")]
fn deserialize_tags<'de, D: serde::Deserializer<'de>>(
    deser: D,
) -> Result<HashMap<OwnedTag, Cow<'de, str>>, D::Error> {
    struct MapVisitor;
    impl<'v> serde::de::Visitor<'v> for MapVisitor {
        type Value = HashMap<OwnedTag, Cow<'v, str>>;

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'v>,
        {
            let mut tags = HashMap::new();

            while let Some((key, value)) = map.next_entry()? {
                tags.insert(key, value);
            }

            Ok(tags)
        }

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map")
        }
    }

    deser.deserialize_map(MapVisitor)
}
#[test]
fn message_builder() {
    use crate::IrcMessage;

    const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";

    let test_parsed: IrcMessage = TEST_MESSAGE.parse().unwrap();

    let built = MessageBuilder::new(IrcCommand::PrivMsg)
        .add_tag(OwnedTag::Unknown("tag1".into()), "val1")
        .add_tag(OwnedTag::Unknown("tag2".into()), "val2")
        .add_tag(OwnedTag::Unknown("tag3".into()), "val3")
        .prefix(OwnedPrefix::Full {
            nickname: "juliapixel".into(),
            username: "julia".into(),
            host: "juliapixel.com".into(),
        })
        .add_param("#juliapixel")
        .add_param(":hi hello there!")
        .build();

    let built_parsed: IrcMessage = built.parse().unwrap();

    assert_eq!(built_parsed, test_parsed);
}
