use std::sync::Arc;

use dashmap::DashMap;
use futures::StreamExt;
use owo_colors::OwoColorize;
use twixel_core::{
    Auth, ConnectionPool, IrcCommand, IrcMessage, MessageBuilder, irc_message::tags::OwnedTag,
};

use crate::command::{Command, CommandContext};

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

fn handle_message(cx: CommandContext<BotCommand>) {
    log::trace!("received message of kind: {}", cx.msg.get_command());
    match cx.msg.get_command() {
        IrcCommand::Ping => {
            cx.bot_tx
                .blocking_send(BotCommand::SendRawIrc(
                    MessageBuilder::pong(cx.msg.get_param(0).unwrap()).to_owned(),
                    cx.connection_idx,
                ))
                .unwrap();
        }
        IrcCommand::AuthSuccessful => {
            log::info!("auth successful")
        }
        IrcCommand::PrivMsg => {
            log::warn!("need to handle PrivMsg");
            let color = cx.msg.get_color().unwrap_or([127, 127, 127]);
            println!(
                "{} {} {}: {:?}",
                cx.msg.get_timestamp().unwrap_or(chrono::Utc::now()),
                cx.msg.get_param(0).unwrap().dimmed(),
                cx.msg
                    .get_tag(OwnedTag::DisplayName)
                    .unwrap()
                    .truecolor(color[0], color[1], color[2]),
                cx.msg.get_param(1).unwrap().split_at(1).1
            );
            cx.bot_tx
                .blocking_send(BotCommand::SendMessage {
                    channel_login: "julialuxel".into(),
                    message: "test".into(),
                    reply_id: cx.msg.get_tag(OwnedTag::Id).map(|s| s.to_owned()),
                })
                .unwrap();
        }
        IrcCommand::Reconnect => {
            cx.bot_tx
                .blocking_send(BotCommand::Reconnect(cx.connection_idx))
                .unwrap();
        }
        IrcCommand::Useless => {
            log::trace!("")
        }
        _ => {
            log::error!("untreated message kind: {:?}", cx.msg.raw())
        }
    }
}

const CMD_CHANNEL_SIZE: usize = 128;

impl Bot {
    pub async fn new(username: String, token: String) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(CMD_CHANNEL_SIZE);
        Self {
            conn_pool: ConnectionPool::new(core::iter::empty::<String>(), Auth::OAuth {
                username,
                token,
            })
            .await
            .unwrap(),
            cmd_rx: rx,
            cmd_tx: tx,
            commands: vec![],
        }
    }

    pub async fn add_channels(
        mut self,
        channels: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for i in channels {
            self.conn_pool.join_channel(i.into()).await.unwrap();
        }
        self
    }

    pub fn add_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    fn get_cmd_cx(&self, msg: IrcMessage<'static>, conn_idx: usize) -> CommandContext<BotCommand> {
        CommandContext {
            msg,
            connection_idx: conn_idx,
            bot_tx: self.cmd_tx.clone(),
            data_store: Arc::new(DashMap::new()),
        }
    }

    pub async fn run(mut self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(recv) = self.conn_pool.next() => {
                        let idx = recv.as_ref().map(|r| r.1).ok();
                        for msg in recv.map(|r| r.0).into_iter().flatten() {
                            let cx = self.get_cmd_cx(msg, idx.unwrap());
                            tokio::task::spawn_blocking(move || { handle_message(cx); });
                        }
                    }
                    cmd = self.cmd_rx.recv() => {
                        match cmd {
                            Some(BotCommand::SendMessage { channel_login, message, reply_id }) => {
                                log::debug!("sending {} to {}", &message, &channel_login);
                                if let Some(idx) = self.conn_pool.get_conn_idx(&channel_login) {
                                    self.conn_pool.send_to_connection(
                                        MessageBuilder::privmsg(&channel_login, &message)
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
                            Some(_) => {
                                todo!("handle other BotCommands!!!")
                            },
                            None => break,
                        }
                    }
                }
            }
        })
        .await
        .unwrap();
    }
}
