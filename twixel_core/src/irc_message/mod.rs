pub mod raw;
pub mod command;
pub mod prefix;
pub mod tags;
pub mod owned;

pub mod error {
    use thiserror::Error;

    use super::{tags::IRCTagParseError, command::IrcCommandError};

    #[derive(Debug, Error)]
    pub enum RawIrcMessageParseError {
        #[error("failed to parse message due to bad tags: {0}")]
        TagParseError(#[from] IRCTagParseError),
        #[error("failed to parse message due to a missing prefix")]
        NoPrefix,
        #[error("failed to parse message due to a missing command")]
        NoCommand,
        #[error(transparent)]
        CommandParseError(#[from] IrcCommandError),
        #[error("failed to parse message due to a missing message")]
        NoMessage,
        #[error(transparent)]
        StructureError(#[from] IrcMessageStructureError),
        #[error("failed to parse message due to it being empty")]
        Empty
    }

    #[derive(Debug, Error)]
    pub enum IrcMessageStructureError {
        #[error("missing separator from tags")]
        MissingTagSeparator,
        #[error("missing separator from prefix")]
        MissingPrefixSeparator,
        #[error("missing final CRLF sequence in message")]
        MissingCrlf
    }

    #[derive(Debug, Error)]
    pub enum IrcMessageParseError {
        #[error("parsing the raw components of message returned an error: {0}")]
        RawParsingError(#[from] RawIrcMessageParseError),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::irc_message::{raw::RawIrcMessage, command::IrcCommand, tags::{RawTag, RawIrcTags, OwnedTag}, owned::OwnedIrcMessage, prefix::OwnedPrefix};

    use super::{error::IrcMessageParseError, prefix::RawPrefix};

    #[test]
    fn raw_message_parsing() {
        const TEST_STR: &str = "@vip=1 :guh PRIVMSG #a :hi there\r\n";

        let parsed = RawIrcMessage::try_from(TEST_STR).unwrap();

        let correct = RawIrcMessage {
            raw: Arc::from(TEST_STR),
            tags: Some(RawIrcTags {
                tags: vec![(RawTag::Vip, 5..6)],
            }),
            prefix: Some(RawPrefix::OnlyHostname { host: 8..11 }),
            command: IrcCommand::PrivMsg,
            params: vec![
                20..22,
                23..32
            ],
        };

        assert_eq!(correct, parsed)
    }

    static SHIT_TON: &'static str = include_str!("../../../logs/logs.txt");

    #[test]
    fn test_a_shit_ton() -> Result<(), IrcMessageParseError> {
        for msg in SHIT_TON.lines() {
            let parsed = RawIrcMessage::try_from(msg);
            assert!(parsed.is_ok(), "failed parsing the following message:\n{msg}")
        }
        Ok(())
    }

    #[test]
    fn raw_tag_parsing() {
        let tag = "display-name";
        assert_eq!(RawTag::DisplayName, RawTag::parse(tag, 0..tag.len()));
    }

    #[test]
    fn raw_prefix_parsing() {
        let full_prefix = ":julia!juliapixel@juliapixel.com FOOBAR";
        let raw_full_prefix = RawPrefix::parse(full_prefix, 1, 32).unwrap();
        let right_full_prefix = RawPrefix::Full {
            nickname: 1..6,
            username: 7..17,
            host: 18..32,
        };

        assert_eq!(raw_full_prefix, right_full_prefix);

        let host_prefix = ":irc.juliapixel.com FOOBAR";
        let raw_host_prefix = RawPrefix::parse(host_prefix, 1, 19).unwrap();
        let right_host_prefix = RawPrefix::OnlyHostname {
            host: 1..19
        };

        assert_eq!(raw_host_prefix, right_host_prefix)
    }

    #[test]
    fn raw_message_display() {
        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi";

        let parsed = RawIrcMessage::try_from(TEST_MESSAGE).unwrap();

        assert_eq!(parsed.to_string(), TEST_MESSAGE);
    }

    #[test]
    fn owned_message_display() {
        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";

        let owned = OwnedIrcMessage {
            tags: Some(vec![
                (OwnedTag::Unknown(String::from("tag1")), String::from("val1")),
                (OwnedTag::Unknown(String::from("tag2")), String::from("val2")),
                (OwnedTag::Unknown(String::from("tag3")), String::from("val3")),
            ]),
            prefix: Some(
                OwnedPrefix::Full {
                    nickname: String::from("juliapixel"),
                    username: String::from("julia"),
                    host: String::from("juliapixel.com")
                }
            ),
            command: IrcCommand::PrivMsg,
            params: vec![
                String::from("#juliapixel"),
                String::from(":hi hello there!")
            ],
        };

        assert_eq!(owned.to_string(), TEST_MESSAGE);
    }

    #[cfg(all(feature = "serde", feature = "unstable"))]
    #[test]
    fn roundtrip_deserialization() {
        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";
        let parsed = RawIrcMessage::try_from(TEST_MESSAGE).unwrap();

        let owned = OwnedIrcMessage {
            tags: Some(vec![
                (OwnedTag::Unknown("tag1".into()), "val1".into()),
                (OwnedTag::Unknown("tag2".into()), "val2".into()),
                (OwnedTag::Unknown("tag3".into()), "val3".into())
            ]),
            prefix: Some(
                OwnedPrefix::Full {
                    nickname: "juliapixel".into(),
                    username: "julia".into(),
                    host: "juliapixel.com".into()
                }
            ),
            command: IrcCommand::PrivMsg,
            params: vec![
                "#juliapixel".into(),
                ":hi hello there!".into()
            ],
        };

        let json_parsed = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json_parsed, serde_json::to_string(&owned).unwrap(), "the OwnedIrcMessage and RawIrcMessage serde::Serialize implementations don't match");

        let deserialized_owned: OwnedIrcMessage = serde_json::from_str(&json_parsed).unwrap();
        assert_eq!(deserialized_owned, owned, "an OwnedIrcMessage could not be deserialized from a serialized RawIrcMessage")
    }
}
