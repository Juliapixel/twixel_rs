use std::{any::Any, sync::Arc};

use dashmap::DashMap;
use futures::StreamExt;
use hashbrown::HashMap;
use tokio::signal::unix::{SignalKind, signal};
use twixel_core::{
    Auth, ConnectionPool, MessageBuilder,
    irc_message::{AnySemantic, PrivMsg, tags::OwnedTag},
};

use crate::{
    anymap::AnyMap,
    guard::GuardContext,
    handler::{Command, CommandContext, CommandHandler},
    util::limit_str_at_graphemes,
};

#[derive(Default, Clone)]
pub struct BotData {
    data: AnyMap,
}

impl BotData {
    fn new() -> Self {
        Self {
            data: AnyMap::new(),
        }
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.data.get::<Arc<T>>().map(|arc| &**arc)
    }

    fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<Arc<T>> {
        self.data.insert::<Arc<T>>(Arc::new(value))
    }

    fn remove<T: Any + Send + Sync>(&mut self) -> Option<T> {
        self.data.remove::<T>()
    }
}

pub struct Bot {
    conn_pool: ConnectionPool,
    commands: Vec<Command>,
    data: BotData,
    cmd_rx: tokio::sync::mpsc::Receiver<BotCommand>,
    cmd_tx: tokio::sync::mpsc::Sender<BotCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    SendMessage {
        channel_login: String,
        message: String,
        reply_id: Option<String>,
    },
    SendRawIrc(MessageBuilder<'static>, usize),
    Reconnect(usize),
    JoinChannel(String),
    PartChannel(String),
    Shutdown,
}

impl BotCommand {
    pub fn respond(msg: &PrivMsg, response: String, reply: bool) -> Self {
        let reply_id = if reply {
            msg.reply_to_id().map(|s| s.to_owned())
        } else {
            None
        };

        Self::SendMessage {
            channel_login: msg.channel_login().into(),
            message: response,
            reply_id,
        }
    }
}

const CMD_CHANNEL_SIZE: usize = 128;

impl Bot {
    pub async fn new(username: String, token: String) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(CMD_CHANNEL_SIZE);
        Self {
            conn_pool: ConnectionPool::new(
                core::iter::empty::<String>(),
                Auth::OAuth { username, token },
            )
            .await
            .unwrap(),
            commands: vec![],
            data: BotData::new(),
            cmd_rx: rx,
            cmd_tx: tx,
        }
    }

    pub async fn add_channels(mut self, channels: impl IntoIterator<Item = &str>) -> Self {
        for i in channels {
            self.conn_pool.join_channel(i).await.unwrap();
        }
        self
    }

    pub fn add_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub fn data<T: Any + Send + Sync>(mut self, data: T) -> Self {
        self.data.insert(data);
        self
    }

    /// Returns whether to shut down or not
    async fn handle_cmd(
        conn_pool: &mut ConnectionPool,
        cmd: BotCommand,
        last_sent_msg: &mut HashMap<String, String>,
    ) -> bool {
        match cmd {
            BotCommand::SendMessage {
                channel_login,
                mut message,
                reply_id,
            } => {
                log::debug!("sending {} to {}", &message, &channel_login);
                if let Some(idx) = conn_pool.get_conn_idx(&channel_login) {
                    let entry = last_sent_msg.entry_ref(&channel_login);
                    entry
                        .and_modify(|v| {
                            if v == &message {
                                message += " \u{e0000}";
                                *v = message.clone();
                            } else {
                                *v = message.clone();
                            }
                        })
                        .or_insert(message.clone());
                    let msg = MessageBuilder::privmsg(
                        &channel_login,
                        limit_str_at_graphemes(&message, 500),
                    )
                    .add_tag(OwnedTag::ReplyParentMsgId, reply_id.unwrap_or_default());
                    conn_pool.send_to_connection(msg, idx).await.unwrap();
                }
            }
            BotCommand::SendRawIrc(raw, idx) => {
                log::debug!("sending {} to connetion {}", raw.command, idx);
                conn_pool.send_to_connection(raw, idx).await.unwrap();
            }
            BotCommand::Reconnect(idx) => {
                conn_pool.restart_connection(idx).await.unwrap();
            }
            BotCommand::JoinChannel(channel) => {
                conn_pool.join_channel(&channel).await.unwrap();
            }
            BotCommand::PartChannel(channel) => {
                conn_pool.part_channel(&channel).await.unwrap();
            }
            BotCommand::Shutdown => {
                log::info!("shutting down");
                return true;
            }
        };
        false
    }

    pub async fn run(mut self) {
        let (tx, rx) = async_channel::bounded(CMD_CHANNEL_SIZE);
        let data_store = Arc::new(self.data);
        let mut msgs = HashMap::<String, String>::new();

        tokio::spawn({
            let tx = self.cmd_tx.clone();
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            async move {
                tokio::select! {
                    _ = sigterm.recv() => {
                        let _ = tx.send(BotCommand::Shutdown).await;
                    }
                    _ = sigint.recv() => {
                        let _ = tx.send(BotCommand::Shutdown).await;
                    }

                }
            }
        });

        let receiver = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle message received from twitch IRC
                    Some(recv) = self.conn_pool.next() => {
                        let idx = recv.as_ref().map(|r| r.1).ok();
                        for msg in recv.map(|r| r.0).into_iter().flatten() {
                            let cx = CommandContext {
                                msg: msg.into(),
                                connection_idx: idx.unwrap(),
                                bot_tx: self.cmd_tx.clone(),
                                data_store: Arc::clone(&data_store)
                            };

                            let new_tx = tx.clone();
                            tokio::spawn(async move { new_tx.send(cx).await.unwrap(); });
                        }
                    }
                    // Handle bot actions
                    cmd = self.cmd_rx.recv() => { match cmd {
                        Some(cmd) => if Self::handle_cmd(&mut self.conn_pool, cmd, &mut msgs).await { break },
                        None => {
                            log::error!("COMMAND CHANNEL BROKEN");
                            break;
                        },
                    }}
                }
            }
        });

        let local_pool = tokio_util::task::LocalPoolHandle::new(
            tokio::runtime::Handle::current().metrics().num_workers(),
        );

        for _i in 0..local_pool.num_threads() {
            let cmds = self.commands.clone();
            let rx = rx.clone();

            local_pool.spawn_pinned({
                // let catchall_clone = self
                //     .catchall
                //     .iter()
                //     .map(|c| c.clone_boxed())
                //     .collect::<Vec<_>>();
                move || bot_worker(rx, cmds)
            });
        }

        if let Err(e) = receiver.await {
            log::error!("{e}");
        }
    }
}

async fn bot_worker(
    rx: async_channel::Receiver<CommandContext>,
    cmds: Vec<Command>,
    // catchall: Vec<Box<dyn CatchallHandler + Send>>,
) {
    while let Ok(cx) = rx.recv().await {
        let msg = cx.msg;
        match &msg {
            AnySemantic::Notice(msg) => {
                match msg.kind() {
                    Some(Ok(k)) => log::info!("received notice of kind: {k}"),
                    Some(Err(_)) => log::error!(
                        "unknown notice kind: {}",
                        msg.get_tag(OwnedTag::MsgId).unwrap()
                    ),
                    None => log::warn!("NOTICE message had no kind"),
                };
                continue;
            }
            AnySemantic::Ping(msg) => {
                cx.bot_tx
                    .send(BotCommand::SendRawIrc(
                        msg.respond().to_owned(),
                        cx.connection_idx,
                    ))
                    .await
                    .unwrap();
                continue;
            }
            AnySemantic::AuthSuccessful(_msg) => {
                log::info!("auth successful");
                continue;
            }
            AnySemantic::Reconnect(_msg) => {
                cx.bot_tx
                    .send(BotCommand::Reconnect(cx.connection_idx))
                    .await
                    .unwrap();
                continue;
            }
            AnySemantic::PrivMsg(_msg) => (),
            AnySemantic::Useless(_msg) => continue,
            AnySemantic::UserState(msg) => {
                log::debug!("received userstate from irc: {:?}", msg.roles());
                continue;
            }
            msg => {
                log::warn!("untreated message kind: {:?}", msg.raw());
                continue;
            }
        }
        let gcx = GuardContext {
            data_store: &Default::default(),
            message: &msg,
        };
        let Some(cmd) = cmds.iter().find(|c| c.matches(&gcx)).cloned() else {
            // for c in &catchall {
            //     if c.handle(cx.clone()).await {
            //         continue;
            //     }
            // }
            continue;
        };
        tokio::task::spawn_local(async move {
            cmd.handle(CommandContext {
                msg,
                connection_idx: cx.connection_idx,
                bot_tx: cx.bot_tx,
                data_store: cx.data_store,
            })
            .await;
        });
    }
}
