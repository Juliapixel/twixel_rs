use std::{
    convert::Infallible,
    future::{Ready, ready},
    time::Duration,
};

use clap::error::{ContextKind, ErrorKind};

pub enum BotResponse {
    Message(String),
    Join(String),
    Part(String),
    Shutdown,
    Many(Vec<Self>),
}

pub trait IntoResponse {
    fn into_response(self) -> impl Future<Output = Option<BotResponse>> + Send;
}

macro_rules! impl_tuple {
    ($([$ty:ident, $num:tt]),+) => {
        impl<$($ty),+> IntoResponse for ($($ty),+)
    where
        $($ty: IntoResponse + Send),+,
    {
        async fn into_response(self) -> Option<BotResponse> {
            Some(
                BotResponse::Many(
                    [$(
                        self.$num.into_response().await,
                    )+].into_iter().filter_map(|r| r).collect()
                )
            )
        }
    }
    };
}

impl_tuple! {[T1, 0], [T2, 1]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2], [T4, 3]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2], [T4, 3], [T5, 4]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2], [T4, 3], [T5, 4], [T6, 5]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2], [T4, 3], [T5, 4], [T6, 5], [T7, 6]}
impl_tuple! {[T1, 0], [T2, 1], [T3, 2], [T4, 3], [T5, 4], [T6, 5], [T7, 6], [T8, 7]}

impl IntoResponse for BotResponse {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        ready(Some(self))
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        ready(None)
    }
}

impl<T: IntoResponse + Send> IntoResponse for Option<T> {
    async fn into_response(self) -> Option<BotResponse> {
        match self {
            Some(s) => s.into_response().await,
            None => None,
        }
    }
}

impl<T: IntoResponse + Send, E: IntoResponse + Send> IntoResponse for Result<T, E> {
    async fn into_response(self) -> Option<BotResponse> {
        match self {
            Ok(o) => o.into_response().await,
            Err(e) => e.into_response().await,
        }
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        unreachable!("WHAT.");
    }
}

impl<T: IntoResponse + Send> IntoResponse for Vec<T> {
    async fn into_response(self) -> Option<BotResponse> {
        Some(BotResponse::Many(
            futures::future::join_all(self.into_iter().map(|r| async { r.into_response().await }))
                .await
                .into_iter()
                .flatten()
                .collect(),
        ))
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        ready(Some(BotResponse::Message(self)))
    }
}

impl IntoResponse for &'_ str {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        ready(Some(BotResponse::Message(self.to_string())))
    }
}

impl IntoResponse for clap::Error {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        if let Some((kind, val)) = self.context().next() {
            if let Some(kind_str) = kind.as_str() {
                return format!("| {kind_str}: {val} | use --help for help").into_response();
            }
            if matches!(kind, ContextKind::Usage) {}
        }
        match self.kind() {
            ErrorKind::InvalidUtf8 => "how did you even send invalid utf8 wtf".into_response(),
            ErrorKind::DisplayHelp => todo!(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => todo!(),
            ErrorKind::DisplayVersion => todo!(),
            _ => ().into_response(),
        }
    }
}

pub struct DelayedResponse<T>(pub T, pub Duration);

impl<T: IntoResponse + Send> IntoResponse for DelayedResponse<T> {
    async fn into_response(self) -> Option<BotResponse> {
        tokio::time::sleep(self.1).await;
        self.0.into_response().await
    }
}
