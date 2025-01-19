//! semantic wrappers around each kind of IRC message command, most of these don't
//! even do anything useful, but are there for completeness' sake

mod clearchat;
mod clearmsg;
mod privmsg;
mod userstate;
mod util;

use crate::IrcMessage;

pub trait SemanticIrcMessage<'a>: Sized {
    fn to_inner(self) -> IrcMessage<'a>
    where
        Self: 'a;

    fn inner(&self) -> &IrcMessage<'a>;

    fn from_message(msg: IrcMessage<'a>) -> Option<Self>
    where
        Self: 'a;
}

macro_rules! impl_semantic {
    ($($cmd:ident),*) => {
        $(
            #[derive(Debug, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            #[cfg_attr(feature = "serde", serde(transparent))]
            #[doc = concat!("a semantic wrapper around a ", stringify!($cmd), " [IrcMessage](super::IrcMessage)")]
            pub struct $cmd<'a> {
                inner: $crate::irc_message::message::IrcMessage<'a>
            }

            impl<'a> ::std::ops::Deref for $cmd<'a> {
                type Target = $crate::irc_message::message::IrcMessage<'a>;

                fn deref(&self) -> &Self::Target {
                    &self.inner
                }
            }

            impl<'a> $crate::irc_message::semantic::SemanticIrcMessage<'a> for $cmd<'a> {
                fn to_inner(self) -> IrcMessage<'a>
                    where Self: 'a {
                    self.inner
                }

                fn inner(&self) -> &$crate::irc_message::message::IrcMessage<'a> {
                    &self.inner
                }

                fn from_message(msg: $crate::irc_message::message::IrcMessage<'a>) -> Option<Self> {
                    if msg.get_command() == $crate::irc_message::command::IrcCommand::$cmd {
                        Some(Self { inner: msg })
                    } else {
                        None
                    }
                }
            }
        )+

        pub enum AnySemantic<'a> {
            $($cmd($cmd<'a>)),+
        }

        impl<'a> From<IrcMessage<'a>> for AnySemantic<'a> {
            fn from(value: IrcMessage<'a>) -> Self {
                match value.get_command() {
                    $($crate::irc_message::command::IrcCommand::$cmd => Self::$cmd($cmd::from_message(value).unwrap()),)+
                    // _ => todo!()
                }
            }
        }

        impl<'a> $crate::irc_message::semantic::SemanticIrcMessage<'a> for AnySemantic<'a> {
            fn to_inner(self) -> IrcMessage<'a>
                where Self: 'a
            {
                match self {
                    $(Self::$cmd(inner) => inner.to_inner()),+
                }
            }


            fn inner(&self) -> &$crate::irc_message::message::IrcMessage<'a> {
                match self {
                    $(Self::$cmd(inner) => inner.inner()),+
                }
            }

            fn from_message(msg: $crate::irc_message::message::IrcMessage<'a>) -> Option<Self> {
                Some(Self::from(msg))
            }
        }
    };
}

impl_semantic!(
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
);
