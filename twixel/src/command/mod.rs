use std::{future::Future, pin::Pin, sync::Arc};

use either::Either;
use guard::CommandGuard;
use twixel_core::irc_message::{PrivMsg, SemanticIrcMessage, Whisper};

use crate::{
    bot::{BotCommand, BotData},
    guard::{AndGuard, Guard, GuardContext, OrGuard},
};

pub mod guard;

pub struct CommandContext<T: SemanticIrcMessage<'static> + 'static> {
    pub msg: T,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<BotCommand>,
    pub data_store: Arc<BotData>,
}

pub trait CommandHandler: 'static {
    fn clone_boxed(&self) -> Box<dyn CommandHandler + Send>;

    fn handle(
        &self,
        cx: CommandContext<Either<PrivMsg, Whisper>>,
    ) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub fn wrap_fn<F, Fut>(func: F) -> impl CommandHandler
where
    F: Fn(CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) -> Fut
        + Clone
        + Send
        + 'static,
    Fut: Future<Output = ()> + 'static,
{
    WrappedHandler { handler: func }
}

#[derive(Clone)]
struct WrappedHandler<H> {
    handler: H,
}

impl<H, Fut> CommandHandler for WrappedHandler<H>
where
    H: Fn(CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) -> Fut
        + Clone
        + Send
        + 'static,
    Fut: Future<Output = ()> + 'static,
{
    fn handle(
        &self,
        cx: CommandContext<Either<PrivMsg, Whisper>>,
    ) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin((self.handler)(cx))
    }

    fn clone_boxed(&self) -> Box<dyn CommandHandler + Send> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct StaticMessageHandler {
    pub msg: String,
}

impl CommandHandler for StaticMessageHandler {
    fn handle(
        &self,
        cx: CommandContext<Either<PrivMsg, Whisper>>,
    ) -> Pin<Box<dyn Future<Output = ()>>> {
        let reply = self.msg.clone();
        Box::pin(async move {
            if let Either::Left(msg) = cx.msg {
                cx.bot_tx
                    .send(BotCommand::respond(&msg, reply, false))
                    .await
                    .unwrap();
            }
        })
    }

    fn clone_boxed(&self) -> Box<dyn CommandHandler + Send> {
        Box::new(self.clone())
    }
}

pub struct Command {
    guard: Box<dyn Guard + Send>,
    pub handler: Box<dyn CommandHandler + Send>,
}

impl Clone for Command {
    fn clone(&self) -> Self {
        Self {
            guard: (*self.guard).clone_boxed(),
            handler: (*self.handler).clone_boxed(),
        }
    }
}

impl Command {
    pub fn new(
        handler: impl CommandHandler + Send,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: Box::new(CommandGuard::new(names, prefix.into())),
        }
    }

    pub async fn handle(&self, cx: CommandContext<Either<PrivMsg<'_>, Whisper<'_>>>) {
        self.handler.handle(cx).await;
    }

    pub fn matches(&self, cx: &GuardContext) -> bool {
        self.guard.check(cx)
    }
}

pub struct CommandBuilder<G: Guard + Clone> {
    pub handler: Box<dyn CommandHandler + Send>,
    guard: G,
}

impl<G: Guard + Clone + Send + Sync> Clone for CommandBuilder<G> {
    fn clone(&self) -> Self {
        Self {
            handler: (*self.handler).clone_boxed(),
            guard: self.guard.clone(),
        }
    }
}

impl CommandBuilder<CommandGuard> {
    pub fn new(
        handler: impl CommandHandler + Send + Sync + 'static,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: CommandGuard::new(names, prefix),
        }
    }
}

impl<G: Guard + Clone + Send + 'static> CommandBuilder<G> {
    pub fn build(self) -> Command {
        Command {
            guard: Box::new(self.guard),
            handler: self.handler,
        }
    }

    pub fn and<G2: Guard + Clone + Send + Sync>(
        self,
        guard: G2,
    ) -> CommandBuilder<AndGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.and(guard),
        }
    }

    pub fn or<G2: Guard + Clone + Send + Sync>(self, guard: G2) -> CommandBuilder<OrGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.or(guard),
        }
    }
}
