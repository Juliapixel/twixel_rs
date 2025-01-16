use std::{
    any::{Any, TypeId},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use dashmap::DashMap;
use twixel_core::{IrcCommand, IrcMessage};

use crate::{
    bot::BotCommand,
    guard::{AndGuard, Guard, GuardContext, OrGuard},
};

pub struct CommandContext<T: Send> {
    pub msg: IrcMessage<'static>,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<T>,
    pub data_store: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

pub trait CommandHandler {
    fn handle(
        &self,
        cx: CommandContext<BotCommand>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;
}

pub fn wrap_fn<F, Fut>(func: F) -> impl CommandHandler
where
    F: Fn(CommandContext<BotCommand>) -> Fut,
    Fut: Future<Output = ()> + Send + Sync + 'static,
{
    WrappedHandler { handler: func }
}

struct WrappedHandler<H> {
    handler: H,
}

impl<H, Fut> CommandHandler for WrappedHandler<H>
where
    H: Fn(CommandContext<BotCommand>) -> Fut,
    Fut: Future<Output = ()> + Send + Sync + 'static,
{
    fn handle(
        &self,
        cx: CommandContext<BotCommand>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        Box::pin((self.handler)(cx))
    }
}

pub struct StaticMessageHandler {
    pub msg: String,
}

impl CommandHandler for StaticMessageHandler {
    fn handle(
        &self,
        cx: CommandContext<BotCommand>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        let msg = self.msg.clone();
        Box::pin(async move {
            cx.bot_tx
                .send(BotCommand::SendMessage {
                    channel_login: cx.msg.get_param(0).unwrap().split_at(1).1.into(),
                    message: msg,
                    reply_id: None,
                })
                .await
                .unwrap();
        })
    }
}

pub struct CommandGuard {
    names: Vec<String>,
    prefix: String,
}

impl CommandGuard {
    pub fn new(names: Vec<String>, prefix: impl Into<String>) -> Self {
        Self {
            names,
            prefix: prefix.into(),
        }
    }
}

impl Guard for CommandGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        if ctx.message().get_command() != IrcCommand::PrivMsg {
            return false;
        }
        let Some((_, msg)) = ctx
            .message()
            .get_param(1)
            .and_then(|m| m.split_at_checked(1))
        else {
            return false;
        };
        let Some((prefix, cmd)) = msg.split_at_checked(1) else {
            return false;
        };
        if prefix != self.prefix {
            return false;
        }
        cmd.split_ascii_whitespace()
            .next()
            .map(|n| self.names.iter().any(|s| s == n))
            .unwrap_or(false)
    }
}

pub struct Command {
    guard: Box<dyn Guard + Send + Sync>,
    pub handler: Box<dyn CommandHandler + Send + Sync>,
}

impl Command {
    pub fn new(
        handler: impl CommandHandler + Send + Sync + 'static,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: Box::new(CommandGuard::new(names, prefix.into())),
        }
    }

    pub async fn handle(&self, cx: CommandContext<BotCommand>) {
        self.handler.handle(cx).await;
    }

    pub fn matches(&self, cx: &GuardContext) -> bool {
        self.guard.check(cx)
    }
}

pub struct CommandBuilder<G: Guard + Send + Sync> {
    pub handler: Box<dyn CommandHandler + Send + Sync>,
    guard: G,
}

impl CommandBuilder<CommandGuard> {
    pub fn new(
        handler: impl CommandHandler + Send + Sync + 'static,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: CommandGuard {
                names,
                prefix: prefix.into(),
            },
        }
    }
}

impl<G: Guard + Send + Sync + 'static> CommandBuilder<G> {
    pub fn build(self) -> Command {
        Command {
            guard: Box::new(self.guard),
            handler: self.handler,
        }
    }

    pub fn and<G2: Guard + Send + Sync>(self, guard: G2) -> CommandBuilder<AndGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.and(guard),
        }
    }

    pub fn or<G2: Guard + Send + Sync>(self, guard: G2) -> CommandBuilder<OrGuard<G, G2>> {
        CommandBuilder {
            handler: self.handler,
            guard: self.guard.or(guard),
        }
    }
}
