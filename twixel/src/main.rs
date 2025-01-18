use bot::{Bot, BotCommand};
use cli::ARGS;
use command::{wrap_fn, Command, CommandBuilder, CommandContext, StaticMessageHandler};
use futures::TryFutureExt;
use guard::{Guard, UserGuard};
use unicode_segmentation::UnicodeSegmentation;

mod bot;
mod cli;
mod command;
mod eval;
mod guard;
mod util;

const JULIA_ID: &str = "173685614";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "INFO"
        },
    ));

    let bot = Bot::new(
        dotenvy::var("TWITCH_LOGIN").unwrap(),
        dotenvy::var("TWITCH_TOKEN").unwrap(),
    )
    .await
    .add_channels(ARGS.channels.iter().map(|s| s.as_str()))
    .await
    .add_command(
        CommandBuilder::new(wrap_fn(join), vec!["join".into()], "%")
            .and(UserGuard::allow(JULIA_ID))
            .build(),
    )
    .add_command(
        CommandBuilder::new(wrap_fn(part), vec!["part".into(), "leave".into()], "%")
            .and(UserGuard::allow(JULIA_ID))
            .build(),
    )
    .add_command(
        CommandBuilder::new(
            eval::EvalHandler::new(),
            vec!["eval".into(), "js".into()],
            "%",
        )
        .and(
            UserGuard::allow(JULIA_ID)
                // ryanpotat
                .or(UserGuard::allow("457260003"))
                // joeiox
                .or(UserGuard::allow("275204234")),
        )
        .build(),
    )
    .add_command(
        CommandBuilder::new(wrap_fn(strdbg), vec!["strdbg".into()], "%")
            .and(UserGuard::allow(JULIA_ID))
            .build(),
    )
    .add_command(Command::new(
        StaticMessageHandler {
            msg: "idk bro figure it out".into(),
        },
        vec!["help".into(), "commands".into()],
        "%",
    ))
    .add_command(Command::new(
        StaticMessageHandler {
            msg: "pong! :3c".into(),
        },
        vec!["ping".into()],
        "%",
    ))
    .add_command(Command::new(wrap_fn(cat_fact), vec!["catfact".into()], "%"));

    log::info!("twixel bot started");

    bot.run().await;
    Ok(())
}

async fn strdbg(cx: CommandContext<BotCommand>) {
    let source_channel: String = cx.msg.get_param(0).unwrap().split_at(1).1.into();
    let Some(msg) = cx
        .msg
        .get_param(1)
        .and_then(|m| m.split_once(' '))
        .map(|(_, m)| m)
    else {
        return;
    };

    cx.bot_tx
        .send(BotCommand::SendMessage {
            channel_login: source_channel,
            message: format!(
                "{} graphemes, {} chars, {} bytes, {:?}",
                msg.graphemes(true).count(),
                msg.chars().count(),
                msg.len(),
                msg
            ),
            reply_id: None,
        })
        .await
        .unwrap()
}

#[derive(serde::Deserialize)]
struct CatFact {
    fact: String,
}

async fn cat_fact(cx: CommandContext<BotCommand>) {
    let resp = match reqwest::get("https://catfact.ninja/fact")
        .and_then(|r| r.json::<CatFact>())
        .await
    {
        Ok(f) => f.fact,
        Err(e) => e.to_string(),
    };

    let source_channel: String = cx.msg.get_param(0).unwrap().split_at(1).1.into();

    cx.bot_tx
        .send(BotCommand::SendMessage {
            channel_login: source_channel,
            message: resp,
            reply_id: None,
        })
        .await
        .unwrap();
}

async fn part(cx: CommandContext<BotCommand>) {
    let Some((Some(_cmd), arg)) = cx.msg.get_param(1).map(|m| m.split_at(1)).map(|(_, m)| {
        let mut splitter = m.split_whitespace();
        (splitter.next(), splitter.next())
    }) else {
        return;
    };

    let source_channel: String = cx.msg.get_param(0).unwrap().split_at(1).1.into();

    match arg {
        Some(chan) => {
            cx.bot_tx
                .send(BotCommand::PartChannel(chan.into()))
                .await
                .unwrap();
            cx.bot_tx
                .send(BotCommand::SendMessage {
                    channel_login: source_channel,
                    message: format!("parting @{chan}"),
                    reply_id: None,
                })
                .await
                .unwrap();
        }
        None => {
            cx.bot_tx
                .send(BotCommand::SendMessage {
                    channel_login: source_channel.clone(),
                    message: "byeeee :333".to_string(),
                    reply_id: None,
                })
                .await
                .unwrap();
            cx.bot_tx
                .send(BotCommand::PartChannel(source_channel.clone()))
                .await
                .unwrap();
        }
    }
}

async fn join(cx: CommandContext<BotCommand>) {
    let Some((Some(_cmd), Some(arg))) = cx.msg.get_param(1).map(|m| m.split_at(1)).map(|(_, m)| {
        let mut splitter = m.split_whitespace();
        (splitter.next(), splitter.next())
    }) else {
        return;
    };

    log::info!("Joining {arg}");

    cx.bot_tx
        .send(BotCommand::JoinChannel(arg.into()))
        .await
        .unwrap();
    cx.bot_tx
        .send(BotCommand::SendMessage {
            channel_login: cx.msg.get_param(0).unwrap().split_at(1).1.into(),
            message: format!("joining @{arg}"),
            reply_id: None,
        })
        .await
        .unwrap();
}
