#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! commands {
    (
        $name:ident, $error:ident,
        [
            $(
                $(#[$comment:meta])*
                $var:ident
            ),+
        ]
        $($key:literal = $val:ident),+
    ) => {
        /// All of Twitch's supported IRC commands
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(into = "&'static str", try_from = "&str"))]
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum $name {
            $(
                $(#[$comment])*
                $var,
            )*
        }

        /// An unidentified IRC command was received
        #[derive(Debug, Clone, PartialEq, Eq, Error)]
        #[error("the IRC command \"{0}\" was not identified!")]
        pub struct $error(String);

        impl TryFrom<&str> for $name {
            type Error = $error;

            fn try_from(val: &str) -> Result<Self, $error> {
                match val {
                    $($key => Ok(Self::$val),)*
                    _ => Err($error(String::from(val)))
                }
            }
        }

        #[allow(unreachable_patterns)]
        impl From<$name> for &str {
            fn from(val: $name) -> &'static str {
                match val {
                    $($name::$val => $key,)*
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", Into::<&str>::into(*self))
            }
        }
    };
}

commands! {
    IrcCommand, IrcCommandError,
    [
        /// The IRC `PASS` command
        Pass,
        /// The IRC `NICK` command
        Nick,
        /// The IRC `JOIN` command
        Join,
        /// The IRC `PART` command
        Part,
        /// The IRC `NOTICE` command
        Notice,
        /// The IRC `CLEARMSG` command
        ClearMsg,
        /// The IRC `CLEARCHAT` command
        ClearChat,
        /// The IRC `HOSTTARGET` command
        HostTarget,
        /// The IRC `PRIVMSG` command
        PrivMsg,
        /// The IRC `PING` command
        Ping,
        /// The IRC `PONG` command
        Pong,
        /// The IRC `CAP` command
        Cap,
        /// The IRC `GLOBALUSERSTATE` command
        GlobalUserState,
        /// The IRC `USERSTATE` command
        UserState,
        /// The IRC `ROOMSTATE` command
        RoomState,
        /// The IRC `USERNOTICE` command
        UserNotice,
        /// The IRC `RECONNECT` command
        Reconnect,
        /// The IRC `WHISPER` command
        Whisper,
        /// The IRC `421` command
        UnsupportedError,
        /// The IRC `353` and `366` commands
        UserList,
        /// The IRC `001` command
        AuthSuccessful,
        /// Many different IRC commands that are sent during twitch's MOTD messages
        Useless
    ]
    "PASS" = Pass,
    "NICK" = Nick,
    "JOIN" = Join,
    "PART" = Part,
    "NOTICE" = Notice,
    "CLEARCHAT" = ClearChat,
    "CLEARMSG" = ClearMsg,
    "HOSTTARGET" = HostTarget,
    "PRIVMSG" = PrivMsg,
    "PING" = Ping,
    "PONG" = Pong,
    "CAP" = Cap,
    "GLOBALUSERSTATE" = GlobalUserState,
    "USERSTATE" = UserState,
    "ROOMSTATE" = RoomState,
    "USERNOTICE" = UserNotice,
    "RECONNECT" = Reconnect,
    "WHISPER" = Whisper,
    "421" = UnsupportedError,
    "353" = UserList,
    "366" = UserList,
    "001" = AuthSuccessful,
    "002" = Useless,
    "003" = Useless,
    "004" = Useless,
    "375" = Useless,
    "372" = Useless,
    "376" = Useless
}
