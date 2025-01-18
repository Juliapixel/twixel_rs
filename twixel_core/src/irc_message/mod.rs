pub mod builder;
pub mod command;
pub mod iter;
pub mod message;
pub mod prefix;
pub mod semantic;
pub mod tags;

pub use command::IrcCommand;
pub use message::IrcMessage;
pub use semantic::*;

pub trait ToIrcMessage {
    fn to_message(self) -> String;

    fn get_command(&self) -> IrcCommand;
}

pub mod error {
    use thiserror::Error;

    use super::{command::IrcCommandError, tags::IRCTagParseError};

    #[derive(Debug, Error)]
    pub enum IrcMessageParseError {
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
        Empty,
    }

    #[derive(Debug, Error)]
    pub enum IrcMessageStructureError {
        #[error("missing separator from tags")]
        MissingTagSeparator,
        #[error("missing separator from prefix")]
        MissingPrefixSeparator,
        #[error("missing final CRLF sequence in message")]
        MissingCrlf,
    }
}

#[cfg(test)]
mod tests {
    use crate::irc_message::{
        builder::MessageBuilder,
        command::IrcCommand,
        message::IrcMessage,
        prefix::OwnedPrefix,
        tags::{OwnedTag, RawTag},
    };

    use super::{error::IrcMessageParseError, prefix::RawPrefix};

    #[test]
    fn raw_message_parsing() {
        const TEST_STR: &str = "@vip=1 :guh PRIVMSG #a :hi there\r\n";

        let parsed: IrcMessage = TEST_STR.parse().unwrap();

        assert_eq!(parsed.get_command(), IrcCommand::PrivMsg);
        assert_eq!(parsed.get_tag(OwnedTag::Vip).unwrap(), "1");
        assert_eq!(parsed.get_param(0).unwrap(), "#a");
        assert_eq!(parsed.get_param(1).unwrap(), ":hi there");
        assert_eq!(parsed.get_host().unwrap(), "guh");
    }

    static SHIT_TON: &str = include_str!("../../../logs/logs.txt");

    #[test]
    fn test_a_shit_ton() -> Result<(), IrcMessageParseError> {
        for msg in SHIT_TON.lines() {
            let parsed: Result<IrcMessage, _> = msg.parse();
            assert!(
                parsed.is_ok(),
                "failed parsing the following message:\n{msg}"
            )
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
        let right_host_prefix = RawPrefix::OnlyHostname { host: 1..19 };

        assert_eq!(raw_host_prefix, right_host_prefix)
    }

    #[test]
    fn raw_message_display() {
        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi";

        let parsed: IrcMessage = TEST_MESSAGE.parse().unwrap();

        assert_eq!(parsed.to_string(), TEST_MESSAGE);
    }

    #[cfg(all(feature = "serde", feature = "unstable"))]
    #[test]
    fn roundtrip_deserialization() {
        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";
        let parsed: IrcMessage = TEST_MESSAGE.parse().unwrap();

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

        let json_parsed = serde_json::to_string(&parsed).unwrap();
        assert_eq!(
            json_parsed,
            serde_json::to_string(&owned).unwrap(),
            "the OwnedIrcMessage and IrcMessage serde::Serialize implementations don't match"
        );

        let deserialized_owned: MessageBuilder =
            serde_json::from_str(&json_parsed).expect(&json_parsed);
        assert_eq!(
            deserialized_owned, owned,
            "an OwnedIrcMessage could not be deserialized from a serialized IrcMessage"
        );

        assert_eq!(
            serde_json::to_string(&deserialized_owned).unwrap(),
            json_parsed
        )
    }
}
