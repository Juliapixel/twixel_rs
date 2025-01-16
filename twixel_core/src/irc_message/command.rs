#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! commands {
    (
        $name:ident, $error:ident,
        [$($var:ident),+]
        $($key:literal = $val:ident),+
    ) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(into = "&'static str", try_from = "&str"))]
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum $name {
            $($var,)*
        }

        #[derive(Debug, Clone, PartialEq, Eq, Error)]
        pub enum $error {
            #[error("the IRC command \"{0}\" was not identified!")]
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
        Pong,
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
