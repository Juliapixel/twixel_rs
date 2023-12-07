pub mod raw;
pub mod command;
pub mod prefix;
pub mod tags;
pub mod message;

pub mod error {
    use thiserror::Error;

    use super::tags::IRCTagParseError;

    #[derive(Debug, Error)]
    pub enum RawIrcMessageParseError {
        #[error("failed to parse message due to bad tags: {0}")]
        TagParseError(#[from] IRCTagParseError),
        #[error("failed to parse message due to a missing prefix")]
        NoPrefix,
        #[error("failed to parse message due to a missing command")]
        NoCommand,
        #[error("failed to parse message due to a missing message")]
        NoMessage,
        #[error("failed to parse message due to a structure error")]
        StructureError,
        #[error("failed to parse message due to it being empty")]
        Empty
    }

    #[derive(Debug, Error)]
    pub enum IrcMessageParseError {
        #[error("parsing the raw components of message returned an error: {0}")]
        RawParsingError(#[from] RawIrcMessageParseError),
    }
}

#[cfg(test)]
mod tests {
    use crate::irc_message::{raw::RawIrcMessage, prefix::Prefix, command::IrcCommand};

    use super::error::IrcMessageParseError;

    #[test]
    fn raw_message_parsing() {
        const TEST_STR: &str = ":julia!juliapixel@juliapixel.com PRIVMSG #juliapixel :hi there!";

        let parsed = RawIrcMessage::try_from(TEST_STR).unwrap();

        let correct = RawIrcMessage {
            tags: None,
            prefix: Some(Prefix::Full {
                nickname: String::from("julia"),
                username: String::from("juliapixel"),
                host: String::from("juliapixel.com")
            }),
            command: IrcCommand::PrivMsg,
            params: vec![
                String::from("#juliapixel"),
                String::from(":hi there!")
            ],
        };

        assert_eq!(correct, parsed)
    }

    static SHIT_TON: &'static str = include_str!("../../logs/logs.txt");

    #[test]
    fn test_a_shit_ton() -> Result<(), IrcMessageParseError> {
        for msg in SHIT_TON.lines() {
            RawIrcMessage::try_from(msg)?;
        }
        Ok(())
    }
}
