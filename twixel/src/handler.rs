use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use extract::{Extract, ExtractFull};
use guard::CommandGuard;
use tokio::sync::mpsc::Sender;
use twixel_core::irc_message::AnySemantic;

use crate::{
    bot::{BotCommand, BotData},
    handler::response::{BotResponse, IntoResponse},
    guard::{AndGuard, Guard, GuardContext, OrGuard},
};

pub mod extract;
pub mod guard;
pub mod response;

#[derive(Clone)]
pub struct CommandContext {
    pub msg: AnySemantic<'static>,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<BotCommand>,
    pub data_store: Arc<BotData>,
}

pub type DynHandler = Pin<
    Box<
        dyn CommandHandler<
                (),
                Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send + 'static>>,
            >,
    >,
>;

pub trait CommandHandler<P>: Send + Sync + 'static {
    type Fut: Future<Output = Option<BotResponse>> + Send + 'static;

    fn clone_boxed(&self) -> DynHandler;

    fn handle(&self, cx: CommandContext) -> Self::Fut;
}

#[derive(Clone, Copy)]
struct ErasedHandler<T, H> {
    handler: H,
    _marker: PhantomData<T>,
}

impl<T, H> CommandHandler<()> for ErasedHandler<T, H>
where
    H: CommandHandler<T> + Send + Sync + Clone,
    T: Send + Sync + 'static,
{
    type Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send + 'static>>;

    fn clone_boxed(&self) -> DynHandler {
        Box::pin(Self {
            handler: self.handler.clone(),
            _marker: PhantomData,
        })
    }

    fn handle(&self, cx: CommandContext) -> Self::Fut {
        Box::pin(self.handler.handle(cx))
    }
}

macro_rules! impl_handler {
    ([$($ty:ident),*], $last:ident) => {
        #[allow(unused_parens, non_snake_case)]
        impl<F, Fut, Res, $($ty,)* $last> CommandHandler<($($ty,)* $last)> for F
        where
            F: FnOnce($($ty,)* $last) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send,
            Res: IntoResponse,
            $($ty: Extract + Send + Sync + 'static,)*
            $last: ExtractFull + Send + Sync + 'static
        {
            type Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send>>;

            fn clone_boxed(&self) -> Pin<Box<dyn CommandHandler<(), Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send + 'static>>>>> {
                Box::pin(ErasedHandler {
                    handler: self.clone(),
                    _marker: PhantomData
                })
            }

            fn handle(
                    &self,
                    cx: CommandContext,
                ) -> Self::Fut {
                let new_self = self.clone();
                Box::pin(async move {
                    let msg = AnySemantic::from(cx.msg);
                    $(
                        let $ty = match $ty::extract(&msg, cx.data_store.clone()).await {
                            Ok(v) => v,
                            Err(e) => return e.into_response()
                        };
                    )*

                    let $last = match $last::extract_full(msg, cx.data_store.clone()).await {
                        Ok(v) => v,
                        Err(e) => return e.into_response()
                    };

                    new_self(
                        $($ty,)*
                        $last
                    ).await.into_response()
                })
            }
        }
    };
}

impl_handler!([], U);
impl_handler!([T1], U);
impl_handler!([T1, T2], U);
impl_handler!([T1, T2, T3], U);
impl_handler!([T1, T2, T3, T4], U);
impl_handler!([T1, T2, T3, T4, T5], U);
impl_handler!([T1, T2, T3, T4, T5, T6], U);
impl_handler!([T1, T2, T3, T4, T5, T6, T7], U);

impl<F, Fut, Res> CommandHandler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
{
    type Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send>>;

    fn clone_boxed(
        &self,
    ) -> Pin<
        Box<
            dyn CommandHandler<
                    (),
                    Fut = Pin<Box<dyn Future<Output = Option<BotResponse>> + Send + 'static>>,
                >,
        >,
    > {
        Box::pin(self.clone())
    }

    fn handle(&self, cx: CommandContext) -> Self::Fut {
        let new_self = self.clone();
        Box::pin(async move { new_self().await.into_response() })
    }
}

pub(crate) fn assert_is_handler<P, T: CommandHandler<P>>(_value: T) {}

#[test]
fn test_handler_trait() {
    async fn wow(username: extract::Username) {
        println!("{}", username.0);
    }

    async fn wow2(_username: extract::Username, _text: extract::MessageText) {}

    async fn hmm() -> String {
        "test".into()
    }

    let erased = ErasedHandler {
        handler: wow2,
        _marker: PhantomData,
    };

    assert_is_handler(wow);
    assert_is_handler(wow2);
    assert_is_handler(hmm);
    assert_is_handler(erased);
}

/// Holds command logic and information, you should use [CommandBuilder](self::CommandBuilder) instead
/// if you plan on adding multiple [Guard](crate::command::Guard)s
pub struct Command {
    guard: Box<dyn Guard>,
    pub handler: DynHandler,
}

impl Clone for Command {
    fn clone(&self) -> Self {
        Self {
            guard: self.guard.clone_boxed(),
            handler: self.handler.clone_boxed(),
        }
    }
}

impl Command {
    pub fn new<T>(
        handler: impl CommandHandler<T>,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: handler.clone_boxed(),
            guard: CommandGuard::new(names, prefix.into()).clone_boxed(),
        }
    }

    async fn handle_resp(resp: BotResponse, privmsg: &AnySemantic<'_>, sender: Sender<BotCommand>) {
        match resp {
            BotResponse::Message(msg) => {
                if let AnySemantic::PrivMsg(privmsg) = privmsg {
                    sender
                        .send(BotCommand::respond(privmsg, msg, false))
                        .await
                        .unwrap();
                }
            }
            BotResponse::Join(chan) => {
                sender.send(BotCommand::JoinChannel(chan)).await.unwrap();
            }
            BotResponse::Part(chan) => {
                sender.send(BotCommand::PartChannel(chan)).await.unwrap();
            }
            BotResponse::Many(bot_responses) => {
                for resp in bot_responses {
                    let sender = sender.clone();
                    Box::pin(
                        async move { Command::handle_resp(resp, privmsg, sender.clone()).await },
                    )
                    .await;
                }
            }
            BotResponse::Shutdown => {
                sender.send(BotCommand::Shutdown).await.unwrap();
            }
        }
    }

    pub async fn handle(&self, cx: CommandContext) {
        let sender = cx.bot_tx.clone();
        if let Some(resp) = self.handler.handle(cx.clone()).await {
            Command::handle_resp(resp, &cx.msg, sender).await
        };
    }

    pub fn matches(&self, cx: &GuardContext) -> bool {
        self.guard.check(cx)
    }
}

pub struct CommandBuilder<T, H: CommandHandler<T>, G: Guard + Clone> {
    pub handler: H,
    guard: G,
    _marker: PhantomData<T>,
}

impl<T, H: CommandHandler<T> + Clone, G: Guard + Clone + Send + Sync> Clone
    for CommandBuilder<T, H, G>
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            guard: self.guard.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, H: CommandHandler<T>> CommandBuilder<T, H, CommandGuard> {
    pub fn new(handler: H, names: Vec<String>, prefix: impl Into<String>) -> Self {
        Self {
            handler,
            guard: CommandGuard::new(names, prefix),
            _marker: PhantomData,
        }
    }
}

impl<T, H: CommandHandler<T>, G: Guard + Clone + Send + 'static> CommandBuilder<T, H, G> {
    pub fn build(self) -> Command {
        Command {
            guard: Box::new(self.guard),
            handler: self.handler.clone_boxed(),
        }
    }

    pub fn and<G2: Guard + Clone + Send + Sync>(
        self,
        guard: G2,
    ) -> CommandBuilder<T, H, AndGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.and(guard),
            _marker: PhantomData,
        }
    }

    pub fn or<G2: Guard + Clone + Send + Sync>(
        self,
        guard: G2,
    ) -> CommandBuilder<T, H, OrGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.or(guard),
            _marker: PhantomData,
        }
    }
}
