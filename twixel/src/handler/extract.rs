use std::{
    convert::Infallible,
    future::{Ready, ready},
    ops::Deref,
    pin::Pin,
    sync::Arc,
};

use futures::FutureExt;
use twixel_core::{
    IrcCommand,
    irc_message::AnySemantic,
};

use crate::{bot::BotData, handler::response::IntoResponse};

/// For extracting data that doesn't requires taking ownership of the message
pub trait Extract: Sized + Send + 'static {
    type Error: IntoResponse + Send + 'static;

    fn extract(
        msg: &AnySemantic<'_>,
        data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

/// For extracting data that requires taking ownership of the message
pub trait ExtractFull: Sized {
    type Error: IntoResponse + Send + 'static;

    fn extract_full(
        msg: AnySemantic<'static>,
        data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

impl ExtractFull for AnySemantic<'static> {
    type Error = Infallible;

    fn extract_full(
        msg: AnySemantic<'static>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        ready(Ok(msg))
    }
}

impl<T: Extract> ExtractFull for T {
    type Error = <T as Extract>::Error;

    async fn extract_full(
        msg: AnySemantic<'static>,
        data: Arc<BotData>,
    ) -> Result<Self, Self::Error> {
        T::extract(&msg, data).await
    }
}

macro_rules! impl_semantic {
    ($($ty:tt),+) => {
        mod semantic {
            $(
                impl super::ExtractFull for twixel_core::irc_message::$ty<'static> {
                    type Error = ();

                    fn extract_full(
                        msg: twixel_core::irc_message::AnySemantic<'static>,
                        _data: std::sync::Arc<crate::bot::BotData>
                    ) -> impl futures::Future<Output = Result<Self, Self::Error>> + std::marker::Send {
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

impl<T: Extract> Extract for Option<T> {
    type Error = Infallible;

    fn extract(
        msg: &AnySemantic<'_>,
        data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        T::extract(msg, data).map(|t| Ok(t.ok()))
    }
}

pub enum Lazy<T: Extract + Send> {
    NotInitialized {
        init: Pin<Box<dyn Future<Output = Result<T, T::Error>> + Send + 'static>>,
    },
    Initialized(Result<T, T::Error>),
}

impl<T: Extract + Send> Lazy<T> {
    pub async fn value(mut self) -> Result<T, <T as Extract>::Error> {
        self.init().await;
        let Self::Initialized(v) = self else {
            unreachable!()
        };
        v
    }

    async fn init(&mut self) {
        if let Self::NotInitialized { init } = self {
            *self = Self::Initialized(init.await)
        }
    }
}

impl<T> Extract for Lazy<T>
where
    T: Extract + Send + 'static,
{
    type Error = Infallible;

    fn extract(msg: &AnySemantic<'_>, data: Arc<BotData>) -> Ready<Result<Self, Infallible>> {
        let msg = msg.clone().to_static();
        let init = Box::pin(async move { T::extract_full(msg, data).await });
        ready(Ok(Self::NotInitialized { init }))
    }
}

pub struct MessageText(pub String);

impl MessageText {
    pub fn split_first_rest(&self) -> Option<(&str, &str)> {
        self.0.split_once(' ')
    }
}

impl Extract for MessageText {
    type Error = ();

    fn extract(
        msg: &AnySemantic<'_>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
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
    type Error = ();

    fn extract(
        msg: &AnySemantic<'_>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
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
    type Error = ();

    fn extract(
        msg: &AnySemantic<'_>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
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
    type Error = ();

    fn extract(
        msg: &AnySemantic<'_>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
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
pub struct Data<T>(pub Arc<T>);

impl<T: Send + Sync + 'static> Extract for Data<T> {
    type Error = Infallible;

    fn extract(
        _msg: &AnySemantic<'_>,
        data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        let data = data.get::<T>().expect("Failed to find data");
        ready(Ok(Self(data)))
    }
}

impl<T> Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extractor for `clap::Parser` implementors
#[derive(Clone, Debug, Copy)]
pub struct Clap<T>(pub T);

impl<T: clap::Parser + Send + 'static> Extract for Clap<T> {
    type Error = Option<clap::Error>;

    fn extract(
        msg: &AnySemantic<'_>,
        _data: Arc<BotData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send {
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
