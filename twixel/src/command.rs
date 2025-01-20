use std::{future::Future, pin::Pin};

use twixel_core::{irc_message::AnySemantic, IrcCommand};

use crate::{
    bot::{BotCommand, BotData},
    guard::{AndGuard, Guard, GuardContext, OrGuard},
};

pub struct CommandContext<T: Send> {
    pub msg: AnySemantic<'static>,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<T>,
    pub data_store: BotData,
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
        if let AnySemantic::PrivMsg(msg) = ctx.message {
            let text = msg.message_text();
            let Some(first_word) = text.split_ascii_whitespace().next() else {
                return false;
            };
            let Some((prefix, cmd)) = first_word.split_at_checked(1) else {
                return false;
            };
            prefix == self.prefix && self.names.iter().any(|name| name == cmd)
        } else {
            false
        }
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
