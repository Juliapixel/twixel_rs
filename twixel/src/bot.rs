use std::sync::Arc;

use dashmap::DashMap;
use futures::StreamExt;
use twixel_core::{
    irc_message::{tags::OwnedTag, AnySemantic}, Auth, ConnectionPool, IrcMessage, MessageBuilder,
};

use crate::{
    command::{Command, CommandContext},
    guard::GuardContext,
    util::limit_str_at_graphemes,
};

pub struct Bot {
    conn_pool: ConnectionPool,
    commands: Vec<Command>,
    cmd_rx: tokio::sync::mpsc::Receiver<BotCommand>,
    cmd_tx: tokio::sync::mpsc::Sender<BotCommand>,
}

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
            cmd_rx: rx,
            cmd_tx: tx,
            commands: vec![],
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

    fn get_cmd_cx(&self, msg: IrcMessage<'static>, conn_idx: usize) -> CommandContext<BotCommand> {
        CommandContext {
            msg: msg.into(),
            connection_idx: conn_idx,
            bot_tx: self.cmd_tx.clone(),
            data_store: Arc::new(DashMap::new()),
        }
    }

    pub async fn run(mut self) {
        let (tx, mut rx) = tokio::sync::mpsc::channel(CMD_CHANNEL_SIZE);
        let cmds = self.commands;
        let receiver = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(recv) = self.conn_pool.next() => {
                        let idx = recv.as_ref().map(|r| r.1).ok();
                        for msg in recv.map(|r| r.0).into_iter().flatten() {
                            let cx = CommandContext {
                                msg: msg.into(),
                                connection_idx: idx.unwrap(),
                                bot_tx: self.cmd_tx.clone(),
                                data_store: Arc::default()
                            };
                            let new_tx = tx.clone();
                            tokio::spawn(async move { new_tx.send(cx).await.unwrap(); });
                        }
                    }
                    cmd = self.cmd_rx.recv() => {
                        match cmd {
                            Some(BotCommand::SendMessage { channel_login, message, reply_id }) => {
                                log::debug!("sending {} to {}", &message, &channel_login);
                                if let Some(idx) = self.conn_pool.get_conn_idx(&channel_login) {
                                    self.conn_pool.send_to_connection(
                                        MessageBuilder::privmsg(&channel_login, limit_str_at_graphemes(&message, 500))
                                            .add_tag(OwnedTag::ReplyParentMsgId, reply_id.unwrap_or_default()),
                                        idx
                                    ).await.unwrap();
                                }
                            },
                            Some(BotCommand::SendRawIrc(raw, idx)) => {
                                log::debug!("sending {} to connetion {}", raw.command, idx);
                                self.conn_pool.send_to_connection(raw, idx).await.unwrap();
                            },
                            Some(BotCommand::Reconnect(idx)) => {
                                self.conn_pool.restart_connection(idx).await.unwrap();
                            },
                            Some(BotCommand::JoinChannel(channel)) => {
                                self.conn_pool.join_channel(&channel).await.unwrap();
                            },
                            Some(BotCommand::PartChannel(channel)) => {
                                self.conn_pool.part_channel(&channel).await.unwrap();
                            },
                            Some(_) => {
                                todo!("handle other BotCommands!!!");
                            },
                            None => {
                                log::error!("COMMAND CHANNEL BROKEN");
                                break;
                            },
                        }
                    }
                }
            }
        });
        let cmd_handler = tokio::spawn(async move {
            let cmds = cmds;
            loop {
                let cx = rx.recv().await.unwrap();
                match &cx.msg {
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
                        log::debug!("received userstate from irc: {:?}", msg.roles())
                    }
                    msg => {
                        log::error!("untreated message kind: {:?}", msg.raw())
                    }
                }
                let gcx = GuardContext { message: &cx.msg };
                let Some(cmd) = cmds.iter().find(|c| c.matches(&gcx)) else {
                    continue;
                };
                cmd.handle(cx).await;
            }
        });
        tokio::select! {
            Err(e) = cmd_handler => {
                log::error!("{e}")
            },
            Err(e) = receiver => {
                log::error!("{e}")
            },
        };
    }
}
