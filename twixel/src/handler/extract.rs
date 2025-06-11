use core::future::Ready;
use std::{convert::Infallible, future::ready, sync::Arc};

use futures::{FutureExt, future::Map};
use twixel_core::{IrcCommand, irc_message::AnySemantic};

use crate::{bot::BotData, handler::response::IntoResponse};

/// For extracting data that doesn't requires taking ownership of the message
pub trait Extract: Sized {
    type Future: Future<Output = Result<Self, Self::Error>> + Send;
    type Error: IntoResponse;

    fn extract(msg: &AnySemantic<'_>, data: Arc<BotData>) -> Self::Future;
}

/// For extracting data that requires taking ownership of the message
pub trait ExtractFull: Sized {
    type Future: Future<Output = Result<Self, Self::Error>> + Send;
    type Error: IntoResponse;

    fn extract_full(msg: AnySemantic<'static>, data: Arc<BotData>) -> Self::Future;
}

impl ExtractFull for AnySemantic<'static> {
    type Future = Ready<Result<Self, Self::Error>>;

    type Error = Infallible;

    fn extract_full(msg: AnySemantic<'static>, _data: Arc<BotData>) -> Self::Future {
        ready(Ok(msg))
    }
}

impl<T: Extract> ExtractFull for T {
    type Future = <T as Extract>::Future;
    type Error = <T as Extract>::Error;

    fn extract_full(msg: AnySemantic<'static>, data: Arc<BotData>) -> Self::Future {
        T::extract(&msg, data)
    }
}

macro_rules! impl_semantic {
    ($($ty:tt),+) => {
        mod semantic {
            $(
                impl super::ExtractFull for twixel_core::irc_message::$ty<'static> {

                    type Future = std::future::Ready<Result<Self, Self::Error>>;

                    type Error = ();

                    fn extract_full(msg: twixel_core::irc_message::AnySemantic<'static>, _data: std::sync::Arc<crate::bot::BotData>) -> Self::Future {
                        std::future::ready(twixel_core::irc_message::$ty::from_any(msg).ok_or(()))
                    }
                }
            )+
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

type ReadySelf<T: Extract> = Ready<Result<T, <T as Extract>::Error>>;

impl<T: Extract> Extract for Option<T> {
    type Future = Map<T::Future, fn(Result<T, T::Error>) -> Result<Option<T>, Infallible>>;

    type Error = Infallible;

    fn extract(msg: &AnySemantic<'_>, data: Arc<BotData>) -> Self::Future {
        T::extract(msg, data).map(|t| Ok(t.ok()))
    }
}

pub struct MessageText(pub String);

impl Extract for MessageText {
    type Future = ReadySelf<Self>;
    type Error = ();

    fn extract(msg: &AnySemantic<'_>, _data: Arc<BotData>) -> Self::Future {
        let text = match msg {
            AnySemantic::PrivMsg(msg) => Ok(Self(
                msg.message_text()
                    .trim_end_matches('\u{e0000}')
                    .trim()
                    .to_owned(),
            )),
            // AnySemantic::Whisper(msg) => {
            //     Some(msg.().to_owned())
            // }
            _ => Err(()),
        };
        ready(text)
    }
}

/// Extractor for sender's login
#[derive(Clone, Debug)]
pub struct Username(pub String);

impl Extract for Username {
    type Future = ReadySelf<Self>;
    type Error = ();

    fn extract(msg: &AnySemantic<'_>, _data: Arc<BotData>) -> Self::Future {
        ready(match msg {
            AnySemantic::PrivMsg(msg) => msg.sender_login().ok_or(()).map(|u| Self(u.to_string())),
            _ => Err(()),
        })
    }
}

/// Extractor for sender's ID
#[derive(Clone, Debug)]
pub struct SenderId(pub String);

impl Extract for SenderId {
    type Future = ReadySelf<Self>;
    type Error = ();

    fn extract(msg: &AnySemantic<'_>, _data: Arc<BotData>) -> Self::Future {
        match msg {
            AnySemantic::PrivMsg(msg) => {
                ready(msg.sender_id().ok_or(()).map(|u| Self(u.to_string())))
            }
            // AnySemantic::Whisper(msg) => ready(msg.sender_id().ok_or(()).map(|u| Self(u.to_string()))),
            _ => ready(Err(())),
        }
    }
}

/// Extractor for source channel's login
#[derive(Clone, Debug)]
pub struct Channel(pub String);

impl Extract for Channel {
    type Future = ReadySelf<Self>;
    type Error = ();

    fn extract(msg: &AnySemantic<'_>, _data: Arc<BotData>) -> Self::Future {
        let chan = if msg.get_command() == IrcCommand::PrivMsg {
            let chan_param = msg
                .get_param(0)
                .expect("no channel param in PrivMsg elisWot");
            if !chan_param.starts_with('#') {
                panic!("channel param malformed")
            } else {
                Some(Self(chan_param.split_at(1).1.to_string()))
            }
        } else {
            None
        };
        core::future::ready(chan.ok_or(()))
    }
}

/// Extractor for bot data
///
/// # Panics
/// if T can't be found
pub struct Data<T>(pub T);

impl<T: Send + Sync + Clone + 'static> Extract for Data<T> {
    type Future = ReadySelf<Self>;
    type Error = Infallible;

    fn extract(_msg: &AnySemantic<'_>, data: Arc<BotData>) -> Self::Future {
        ready(Ok(Self(
            data.get::<T>().expect("Failed to find data").clone(),
        )))
    }
}

/// Extractor for `clap::Parser` implementors
#[derive(Clone, Debug, Copy)]
pub struct Clap<T>(pub T);

impl<T: clap::Parser + Send> Extract for Clap<T> {
    type Future = ReadySelf<Self>;

    type Error = Option<clap::Error>;

    fn extract(msg: &AnySemantic<'_>, _data: Arc<BotData>) -> Self::Future {
        let AnySemantic::PrivMsg(msg) = msg else {
            return ready(Err(None));
        };
        let segments = msg
            .message_text()
            .split('"')
            .enumerate()
            .flat_map(|(i, v)| {
                if i % 2 == 0 {
                    v.split_ascii_whitespace().collect::<Vec<_>>()
                } else {
                    vec![v]
                }
            });
        ready(match T::try_parse_from(segments) {
            Ok(t) => Ok(Self(t)),
            Err(e) => Err(Some(e)),
        })
    }
}
