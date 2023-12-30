#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use thiserror::Error;

macro_rules! commands {
    (
        $name:ident, $error:ident,
        [$($var:ident),+]
        $($key:literal = $val:ident),+
    ) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(into = "&'static str"))]
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum $name {
            $($var,)*
        }

        #[derive(Debug, Clone, PartialEq, Eq, Error)]
        pub enum $error {
            #[error("the IRC command was not identified!")]
            Failed(String),
        }

        impl TryFrom<&str> for $name {
            type Error = $error;

            fn try_from(val: &str) -> Result<Self, $error> {
                match val {
                    $($key => Ok(Self::$val),)*
                    _ => Err($error::Failed(String::from(val)))
                }
            }
        }

        impl From<$name> for &str {
            fn from(val: $name) -> &'static str {
                match val {
                    $($name::$val => $key,)*
                }
            }
        }
    };
}

commands!{
    IrcCommand, IrcCommandError,
    [
        Pass,
        Nick,
        Join,
        Part,
        Notice,
        ClearMsg,
        ClearChat,
        HostTarget,
        PrivMsg,
        Ping,
        Cap,
        GlobalUserState,
        UserState,
        RoomState,
        UserNotice,
        Reconnect,
        Whisper,
        UnsupportedError,
        UserList,
        AuthSuccessful,
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
