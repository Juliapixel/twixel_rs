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

pub trait SemanticIrcMessage<'a>: Sized {
    fn to_inner(self) -> IrcMessage<'a>
    where
        Self: 'a;

    fn inner(&self) -> &IrcMessage<'a>;

    #[allow(clippy::result_large_err, reason = "intended")]
    fn from_message(msg: IrcMessage<'a>) -> Result<Self, IrcMessage<'a>>
    where
        Self: 'a;
}

impl<'a, L, R> SemanticIrcMessage<'a> for either::Either<L, R>
where
    L: SemanticIrcMessage<'a>,
    R: SemanticIrcMessage<'a>,
{
    fn to_inner(self) -> IrcMessage<'a>
    where
        Self: 'a,
    {
        match self {
            either::Either::Left(l) => l.to_inner(),
            either::Either::Right(r) => r.to_inner(),
        }
    }

    fn inner(&self) -> &IrcMessage<'a> {
        match self {
            either::Either::Left(l) => l.inner(),
            either::Either::Right(r) => r.inner(),
        }
    }

    fn from_message(msg: IrcMessage<'a>) -> Result<Self, IrcMessage<'a>>
    where
        Self: 'a,
    {
        match L::from_message(msg) {
            Ok(l) => Ok(Either::Left(l)),
            Err(m) => R::from_message(m).map(|r| Either::Right(r)),
        }
    }
}

macro_rules! impl_semantic {
    ($($cmd:ident),*) => {
        $(
            #[derive(Debug, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            #[cfg_attr(feature = "serde", serde(transparent))]
            #[doc = concat!("a semantic wrapper around a ", stringify!($cmd), " [IrcMessage](super::message::IrcMessage)")]
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

                fn from_message(msg: $crate::irc_message::message::IrcMessage<'a>) -> Result<Self, IrcMessage<'a>> {
                    if msg.get_command() == $crate::irc_message::command::IrcCommand::$cmd {
                        Ok(Self { inner: msg })
                    } else {
                        Err(msg)
                    }
                }
            }

            impl<'a> $cmd<'a> {
                pub fn from_any(any: AnySemantic<'a>) -> Option<Self> {
                    match any {
                        AnySemantic::$cmd(c) => Some(c),
                        _ => None
                    }
                }

                pub fn from_any_ref(any: &'a AnySemantic<'a>) -> Option<&'a Self> {
                    match any {
                        AnySemantic::$cmd(c) => Some(c),
                        _ => None
                    }
                }
            }
        )+

        /// enum containing all semantic wrappers around [IrcMessage](super::message::IrcMessage)
        pub enum AnySemantic<'a> {
            $($cmd($cmd<'a>)),+
        }

        impl<'a> ::std::ops::Deref for AnySemantic<'a> {
            type Target = $crate::irc_message::message::IrcMessage<'a>;

            fn deref(&self) -> &Self::Target {
                &self.inner()
            }
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

            fn from_message(msg: $crate::irc_message::message::IrcMessage<'a>) -> Result<Self, IrcMessage<'a>> {
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

impl Display for AnySemantic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.inner().raw())
    }
}
