use std::{
    any::{Any, TypeId},
    future::Future,
    sync::Arc,
};

use dashmap::DashMap;
use twixel_core::{IrcCommand, IrcMessage};

use crate::{
    bot::BotCommand,
    guard::{Guard, GuardContext},
};

pub struct CommandContext<T: Send> {
    pub msg: IrcMessage<'static>,
    pub connection_idx: usize,
    pub bot_tx: tokio::sync::mpsc::Sender<T>,
    pub data_store: Arc<DashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

pub trait CommandHandler {
    fn handle(&self, cx: CommandContext<BotCommand>) -> Box<dyn Future<Output = ()>>;
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
        if let Some((_, msg)) = ctx
            .message()
            .get_param(1)
            .and_then(|m| m.split_at_checked(1))
        {
            msg.starts_with(&self.prefix)
                && msg
                    .split_at_checked(1)
                    .and_then(|s| {
                        s.1.split_ascii_whitespace()
                            .next()
                            .map(|f| self.prefix.contains(f))
                    })
                    .unwrap_or(false)
        } else {
            false
        }
    }
}

pub struct Command {
    guard: Box<dyn Guard + Send>,
    handler: Box<dyn CommandHandler + Send>,
}

impl Command {
    pub fn new(
        handler: impl CommandHandler + Send + 'static,
        names: Vec<String>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: Box::new(CommandGuard {
                names,
                prefix: prefix.into(),
            }),
        }
    }

    pub fn new_with_guard(
        handler: impl CommandHandler + Send + 'static,
        guard: impl Guard + Send + 'static,
    ) -> Self {
        Self {
            handler: Box::new(handler),
            guard: Box::new(guard),
        }
    }
}
