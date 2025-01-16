use std::{borrow::Cow, fmt::Write};

use super::{command::IrcCommand, prefix::OwnedPrefix, tags::OwnedTag, ToIrcMessage};

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MessageBuilder<'a> {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_tags"))]
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_tags"))]
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub tags: Vec<(OwnedTag, Cow<'a, str>)>,
    pub prefix: Option<OwnedPrefix>,
    pub command: IrcCommand,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub params: Vec<Cow<'a, str>>,
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
    pub fn new(command: IrcCommand) -> Self {
        Self {
            tags: vec![],
            prefix: None,
            command,
            params: vec![],
        }
    }

    pub fn build(self) -> String {
        let mut out = String::new();

        // tags
        for (idx, tag) in self.tags.iter().enumerate() {
            if idx == 0 {
                write!(&mut out, "@").unwrap();
            } else {
                write!(&mut out, ";").unwrap();
            }
            write!(
                &mut out,
                "{}={}",
                Into::<&str>::into(&tag.0),
                tag.1
            ).unwrap()
        }

        if !self.tags.is_empty() {
            write!(&mut out, " ").unwrap();
        }

        // prefix
        if let Some(prefix) = &self.prefix {
            write!(&mut out, "{} ", prefix).unwrap();
        }

        // command
        write!(&mut out, "{}", self.command).unwrap();

        // params
        for param in self.params.into_iter() {
            write!(
                &mut out,
                " {}",
                param,
            )
            .unwrap();
        }

        // CRLF EOL
        write!(&mut out, "\r\n").unwrap();

        out
    }

    pub fn add_tag(mut self, tag: OwnedTag, value: impl Into<Cow<'a, str>>) -> Self {
        self.tags.push((tag, value.into()));
        self
    }

    pub fn add_param(mut self, param: impl Into<Cow<'a, str>>) -> Self {
        self.params.push(param.into());
        self
    }

    pub fn prefix(mut self, prefix: OwnedPrefix) -> Self {
        let _ = self.prefix.insert(prefix);
        self
    }

    pub fn privmsg(channel: &'a str, message: &str) -> Self {
        let chan_param = if channel.starts_with('#') {
            Cow::Borrowed(channel)
        } else {
            Cow::Owned(format!("#{channel}"))
        };
        Self::new(IrcCommand::PrivMsg)
            .add_param(chan_param)
            .add_param(Cow::Owned(format!(":{message}")))
    }

    pub fn pong(data: &'a str) -> Self {
        Self::new(IrcCommand::Pong).add_param(Cow::Borrowed(data))
    }

    pub fn join(channels: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        let mut channel_list = String::new();
        for (idx, chan) in channels.into_iter().enumerate() {
            if idx > 0 {
                write!(&mut channel_list, ",").unwrap()
            }
            write!(&mut channel_list, "#{}", chan).unwrap()
        }
        Self::new(IrcCommand::Join).add_param(channel_list)
    }

    pub fn cap_req() -> Self {
        Self::new(IrcCommand::Cap)
            .add_param(Cow::Borrowed("REQ"))
            .add_param(Cow::Borrowed(":twitch.tv/commands twitch.tv/tags"))
    }

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
    value: &Vec<(OwnedTag, Cow<'_, str>)>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    ser.collect_map(value.iter().map(|(k, v)| (k, v)))
}

#[cfg(feature = "serde")]
fn deserialize_tags<'de, D: serde::Deserializer<'de>>(
    deser: D,
) -> Result<Vec<(OwnedTag, Cow<'de, str>)>, D::Error> {
    struct MapVisitor;
    impl<'v> serde::de::Visitor<'v> for MapVisitor {
        type Value = Vec<(OwnedTag, Cow<'v, str>)>;

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'v>,
        {
            let mut tags = Vec::new();

            while let Some((key, value)) = map.next_entry()? {
                tags.push((key, value));
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
    const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";

    let owned = MessageBuilder::new(IrcCommand::PrivMsg)
        .add_tag(OwnedTag::Unknown("tag1".into()), "val1")
        .add_tag(OwnedTag::Unknown("tag2".into()), "val2")
        .add_tag(OwnedTag::Unknown("tag3".into()), "val3")
        .prefix(OwnedPrefix::Full {
            nickname: "juliapixel".into(),
            username: "julia".into(),
            host: "juliapixel.com".into(),
        })
        .add_param("#juliapixel")
        .add_param(":hi hello there!");

    assert_eq!(owned.build(), TEST_MESSAGE);
}
