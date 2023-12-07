#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(into = "String"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IrcCommand {
    Join,
    Part,
    Notice,
    ClearChat,
    ClearMsg,
    HostTarget,
    PrivMsg,
    Whisper,
    Ping,
    Cap,
    GlobalUserState,
    UserState,
    RoomState,
    UserNotice,
    Reconnect,
    UnsupportedError,
    AuthSuccessful,
    UserList,
    Useless,
}

#[derive(Debug)]
pub enum IrcCommandError {
    Failed,
}

impl TryFrom<&str> for IrcCommand {
    type Error = IrcCommandError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        match input {
            "JOIN" => Ok(Self::Join),
            "PART" => Ok(Self::Part),
            "NOTICE" => Ok(Self::Notice),
            "CLEARCHAT" => Ok(Self::ClearChat),
            "CLEARMSG" => Ok(Self::ClearMsg),
            "HOSTTARGET" => Ok(Self::HostTarget),
            "PRIVMSG" => Ok(Self::PrivMsg),
            "PING" => Ok(Self::Ping),
            "CAP" => Ok(Self::Cap),
            "GLOBALUSERSTATE" => Ok(Self::GlobalUserState),
            "USERSTATE" => Ok(Self::UserState),
            "ROOMSTATE" => Ok(Self::RoomState),
            "USERNOTICE" => Ok(Self::UserNotice),
            "RECONNECT" => Ok(Self::Reconnect),
            "WHISPER" => Ok(Self::Whisper),
            "421" => Ok(Self::UnsupportedError),
            "353" => Ok(Self::UserList),
            "366" => Ok(Self::UserList),
            "001" => Ok(Self::AuthSuccessful),
            "002" => Ok(Self::Useless),
            "003" => Ok(Self::Useless),
            "004" => Ok(Self::Useless),
            "375" => Ok(Self::Useless),
            "372" => Ok(Self::Useless),
            "376" => Ok(Self::Useless),
             _ => Err(IrcCommandError::Failed),
            }
    }
}

impl From<IrcCommand> for String {
    fn from(value: IrcCommand) -> Self {
        return String::from(match value {
            IrcCommand::Join => "JOIN",
            IrcCommand::Part => "PART",
            IrcCommand::Notice => "NOTICE",
            IrcCommand::ClearChat => "CLEARCHAT",
            IrcCommand::ClearMsg => "CLEARMSG",
            IrcCommand::HostTarget => "HOSTTARGET",
            IrcCommand::PrivMsg => "PRIVMSG",
            IrcCommand::Whisper => "WHISPER",
            IrcCommand::Ping => "PING",
            IrcCommand::Cap => "CAP",
            IrcCommand::GlobalUserState => "GLOBALUSERSTATE",
            IrcCommand::UserState => "USERSTATE",
            IrcCommand::RoomState => "ROOMSTATE",
            IrcCommand::UserNotice => "USERNOTICE",
            IrcCommand::Reconnect => "RECONNECT",
            IrcCommand::UnsupportedError => "421",
            IrcCommand::AuthSuccessful => "001",
            IrcCommand::UserList => "353",
            IrcCommand::Useless => "",
        })
    }
}
