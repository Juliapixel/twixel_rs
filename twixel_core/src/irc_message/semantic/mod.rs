//! semantic wrappers around each kind of IRC message command, most of these don't
//! even do anything useful, but are there for completeness' sake

mod clearchat;
mod clearmsg;
mod notice;
mod ping;
mod privmsg;
mod userstate;
mod util;

use std::fmt::Display;

use either::Either;

use crate::IrcMessage;

pub trait SemanticIrcMessage: Sized + private::Sealed {
    fn to_inner(self) -> IrcMessage;

    fn inner(&self) -> &IrcMessage;

    #[allow(clippy::result_large_err, reason = "intended")]
    fn from_message(msg: IrcMessage) -> Result<Self, IrcMessage>;
}

mod private {
    pub trait Sealed {}
}

impl<L, R> private::Sealed for either::Either<L, R>
where
    L: SemanticIrcMessage,
    R: SemanticIrcMessage,
{}

impl<L, R> SemanticIrcMessage for either::Either<L, R>
where
    L: SemanticIrcMessage,
    R: SemanticIrcMessage,
{
    fn to_inner(self) -> IrcMessage {
        match self {
            either::Either::Left(l) => l.to_inner(),
            either::Either::Right(r) => r.to_inner(),
        }
    }

    fn inner(&self) -> &IrcMessage {
        match self {
            either::Either::Left(l) => l.inner(),
            either::Either::Right(r) => r.inner(),
        }
    }

    fn from_message(msg: IrcMessage) -> Result<Self, IrcMessage> {
        match L::from_message(msg) {
            Ok(l) => Ok(Either::Left(l)),
            Err(m) => R::from_message(m).map(|r| Either::Right(r)),
        }
    }
}

macro_rules! impl_semantic {
    ($($cmd:ident),*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            #[cfg_attr(feature = "serde", serde(transparent))]
            #[doc = concat!("a semantic wrapper around a [", stringify!($cmd), "](crate::IrcCommand::", stringify!($cmd), ") [IrcMessage](super::message::IrcMessage)")]
            pub struct $cmd {
                inner: $crate::irc_message::message::IrcMessage
            }

            impl ::std::ops::Deref for $cmd {
                type Target = $crate::irc_message::message::IrcMessage;

                fn deref(&self) -> &Self::Target {
                    &self.inner
                }
            }

            impl private::Sealed for $cmd {}

            impl $crate::irc_message::semantic::SemanticIrcMessage for $cmd {
                fn to_inner(self) -> IrcMessage {
                    self.inner
                }

                fn inner(&self) -> &$crate::irc_message::message::IrcMessage {
                    &self.inner
                }

                fn from_message(msg: $crate::irc_message::message::IrcMessage) -> Result<Self, IrcMessage> {
                    if msg.get_command() == $crate::irc_message::command::IrcCommand::$cmd {
                        Ok(Self { inner: msg })
                    } else {
                        Err(msg)
                    }
                }
            }

            impl $cmd {
                /// Tries to convert from [AnySemantic] to this type
                pub fn from_any(any: AnySemantic) -> Option<Self> {
                    match any {
                        AnySemantic::$cmd(c) => Some(c),
                        _ => None
                    }
                }

                /// Tries to convert from [&AnySemantic](AnySemantic) to a reference to this type
                pub fn from_any_ref(any: &AnySemantic) -> Option<&Self> {
                    match any {
                        AnySemantic::$cmd(c) => Some(c),
                        _ => None
                    }
                }
            }
        )+

        /// enum containing all semantic wrappers around [crate::IrcMessage]
        #[derive(Debug, Clone)]
        pub enum AnySemantic {
            $($cmd($cmd)),+
        }

        impl ::std::ops::Deref for AnySemantic {
            type Target = $crate::irc_message::message::IrcMessage;

            fn deref(&self) -> &Self::Target {
                &self.inner()
            }
        }

        impl From<IrcMessage> for AnySemantic {
            fn from(value: IrcMessage) -> Self {
                match value.get_command() {
                    $($crate::irc_message::command::IrcCommand::$cmd => Self::$cmd($cmd::from_message(value).unwrap()),)+
                    // _ => todo!()
                }
            }
        }

        impl private::Sealed for AnySemantic {}

        impl $crate::irc_message::semantic::SemanticIrcMessage for AnySemantic {
            fn to_inner(self) -> IrcMessage {
                match self {
                    $(Self::$cmd(inner) => inner.to_inner()),+
                }
            }

            fn inner(&self) -> &$crate::irc_message::message::IrcMessage {
                match self {
                    $(Self::$cmd(inner) => inner.inner()),+
                }
            }

            fn from_message(msg: $crate::irc_message::message::IrcMessage) -> Result<Self, IrcMessage> {
                Ok(Self::from(msg))
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

impl Display for AnySemantic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.inner().inner())
    }
}
