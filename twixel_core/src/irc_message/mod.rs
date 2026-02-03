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

/// Trait for types which can be sent over an IRC connection
pub trait ToIrcMessage {
    /// Convert to a valid IRC message
    fn to_message(self) -> String;

    /// Get the message's IRC command
    fn get_command(&self) -> IrcCommand;
}

/// Error types associated with [IrcMessage] and related operations
pub mod error {
    use thiserror::Error;

    use super::{command::IrcCommandError, tags::IRCTagParseError};

    /// Errors that may occur when parsing an [IrcMessage](crate::IrcMessage)
    #[derive(Debug, Error)]
    pub enum IrcMessageParseError {
        /// Error when parsing the IRCv3 tags of the message
        #[error("failed to parse message due to bad tags: {0}")]
        TagParseError(#[from] IRCTagParseError),
        /// The message's prefix could not be found
        #[error("failed to parse message due to a missing prefix")]
        NoPrefix,
        /// The message's command could not be found
        #[error("failed to parse message due to a missing command")]
        NoCommand,
        /// The message's command could not be parsed
        #[error(transparent)]
        CommandParseError(#[from] IrcCommandError),
        /// There was no message to parse
        #[error("failed to parse message due to a missing message")]
        NoMessage,
        /// There was an error while parsing the message's structure
        #[error(transparent)]
        StructureError(#[from] IrcMessageStructureError),
        /// The provided message was an empty string
        #[error("failed to parse message due to it being empty")]
        Empty,
    }

    /// Structural errors that may occur when parsing an [IrcMessage](crate::IrcMessage)
    #[derive(Debug, Error)]
    pub enum IrcMessageStructureError {
        /// There was no space character separating the tags segment from the rest
        /// of the message
        #[error("missing separator from tags")]
        MissingTagSeparator,
        /// There was no space character separating the prefix segment from the rest
        /// of the message
        #[error("missing separator from prefix")]
        MissingPrefixSeparator,
        /// There was no CRLF sequence at the end of the message
        #[error("missing final CRLF sequence in message")]
        MissingCrlf,
    }
}

#[cfg(test)]
mod tests {
    use crate::irc_message::{
        command::IrcCommand,
        message::IrcMessage,
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
        assert_eq!(parsed.get_param(1).unwrap(), "hi there");
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
        use crate::MessageBuilder;

        const TEST_MESSAGE: &str = "@tag1=val1;tag2=val2;tag3=val3 :juliapixel!julia@juliapixel.com PRIVMSG #juliapixel :hi hello there!\r\n";
        let parsed: IrcMessage = TEST_MESSAGE.parse().unwrap();

        let json_parsed = serde_json::to_string(&parsed).unwrap();

        let deserialized_owned: MessageBuilder =
            serde_json::from_str(&json_parsed).expect(&json_parsed);

        let rebuilt = IrcMessage::new(deserialized_owned.build()).unwrap();
        assert_eq!(
            parsed, rebuilt,
            "an OwnedIrcMessage could not be deserialized from a serialized IrcMessage"
        );
    }
}
