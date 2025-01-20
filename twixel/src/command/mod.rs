use std::{future::Future, pin::Pin, sync::Arc};

use guard::CommandGuard;
use twixel_core::irc_message::AnySemantic;

use crate::{
    bot::{BotCommand, BotData},
    guard::{AndGuard, Guard, GuardContext, OrGuard},
};

pub mod guard;

pub struct CommandContext<T: Send> {
    pub msg: AnySemantic<'static>,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<T>,
    pub data_store: Arc<BotData>,
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
        let reply = self.msg.clone();
        Box::pin(async move {
            if let AnySemantic::PrivMsg(msg) = cx.msg {
                cx.bot_tx
                    .send(BotCommand::SendMessage {
                        channel_login: msg.channel_login().to_owned(),
                        message: reply,
                        reply_id: None,
                    })
                    .await
                    .unwrap();
            }
        })
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
            guard: CommandGuard::new(names, prefix),
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
