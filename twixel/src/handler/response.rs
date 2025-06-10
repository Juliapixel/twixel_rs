use clap::error::{ContextKind, ErrorKind};

pub enum BotResponse {
    Message(String),
    Join(String),
    Part(String),
    Shutdown,
    Many(Vec<Self>),
}

pub trait IntoResponse {
    fn into_response(self) -> Option<BotResponse>;
}

macro_rules! impl_tuple {
    ($([$ty:ident, $num:tt]),+) => {
        impl<$($ty),+> IntoResponse for ($($ty),+)
    where
        $($ty: IntoResponse),+,
    {
        fn into_response(self) -> Option<BotResponse> {
            Some(
                BotResponse::Many(
                    [$(
                        self.$num.into_response(),
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
    fn into_response(self) -> Option<BotResponse> {
        Some(self)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Option<BotResponse> {
        None
    }
}

impl<T: IntoResponse> IntoResponse for Option<T> {
    fn into_response(self) -> Option<BotResponse> {
        self.and_then(|t| t.into_response())
    }
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Option<BotResponse> {
        match self {
            Ok(o) => o.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl<T: IntoResponse> IntoResponse for Vec<T> {
    fn into_response(self) -> Option<BotResponse> {
        Some(BotResponse::Many(
            self.into_iter().filter_map(|r| r.into_response()).collect(),
        ))
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Option<BotResponse> {
        Some(BotResponse::Message(self))
    }
}

impl IntoResponse for &'_ str {
    fn into_response(self) -> Option<BotResponse> {
        Some(BotResponse::Message(self.to_string()))
    }
}

impl IntoResponse for clap::Error {
    fn into_response(self) -> Option<BotResponse> {
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
            _ => None,
        }
    }
}
