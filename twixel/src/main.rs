use bot::{Bot, BotCommand};
use cli::ARGS;
use command::{wrap_fn, Command, CommandBuilder, CommandContext, StaticMessageHandler};
use config::CONFIG;
use futures::TryFutureExt;
use guard::{RoleGuard, UserGuard};
use twixel_core::{
    irc_message::{tags::OwnedTag, AnySemantic},
    user::ChannelRoles,
};
use unicode_segmentation::UnicodeSegmentation;

mod anymap;
mod bot;
mod cli;
mod command;
mod config;
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

    let bot = Bot::new(CONFIG.twitch.login.clone(), CONFIG.twitch.token.clone())
        .await
        .add_channels(ARGS.channels.iter().map(|s| s.as_str()))
        .await
        .data(String::from("global data is here"))
        .add_command(
            CommandBuilder::new(wrap_fn(join), vec!["join".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        .add_command(
            CommandBuilder::new(wrap_fn(mod_only), vec!["modonly".into()], "%")
                .and(RoleGuard::new(
                    ChannelRoles::Moderator | ChannelRoles::Broadcaster,
                ))
                .build(),
        )
        .add_command(
            CommandBuilder::new(wrap_fn(part), vec!["part".into(), "leave".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        .add_command(
            CommandBuilder::new(
                eval::EvalHandler::new(),
                vec!["eval".into(), "js".into()],
                "%",
            )
            .and(UserGuard::allow([JULIA_ID, "457260003", "275204234"]))
            .build(),
        )
        .add_command(
            CommandBuilder::new(wrap_fn(strdbg), vec!["strdbg".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
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

async fn mod_only(cx: CommandContext<BotCommand>) {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return;
    };

    let user_name = msg.get_tag(OwnedTag::DisplayName).unwrap();

    let reply = if msg.sender_roles().intersects(ChannelRoles::Broadcaster) {
        format!("@{user_name} is the broadcaster wowie !!!")
    } else {
        format!("@{user_name} is a mod wowie !!!")
    };

    cx.bot_tx
        .send(BotCommand::respond(&msg, reply, false))
        .await
        .unwrap();
}

async fn strdbg(cx: CommandContext<BotCommand>) {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return;
    };

    let response = format!(
        "{} graphemes, {} chars, {} bytes, {:?}",
        msg.message_text().graphemes(true).count(),
        msg.message_text().chars().count(),
        msg.message_text().len(),
        msg.message_text()
    );

    cx.bot_tx
        .send(BotCommand::respond(&msg, response, false))
        .await
        .unwrap()
}

#[derive(serde::Deserialize)]
struct CatFact {
    fact: String,
}

async fn cat_fact(cx: CommandContext<BotCommand>) {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return;
    };

    let resp = match reqwest::get("https://catfact.ninja/fact")
        .and_then(|r| r.json::<CatFact>())
        .await
    {
        Ok(f) => f.fact,
        Err(e) => e.to_string(),
    };

    cx.bot_tx
        .send(BotCommand::respond(&msg, resp, false))
        .await
        .unwrap();
}

async fn part(cx: CommandContext<BotCommand>) {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return;
    };

    let args = msg
        .message_text()
        .split_ascii_whitespace()
        .skip(1)
        .collect::<Vec<_>>();

    let source_channel = msg.channel_login();

    if args.is_empty() {
        let source_channel = source_channel.to_owned();
        cx.bot_tx
            .send(BotCommand::respond(&msg, "byeeee :333".into(), false))
            .await
            .unwrap();
        cx.bot_tx
            .send(BotCommand::PartChannel(source_channel))
            .await
            .unwrap();
    } else {
        let channels = args.join(", ");

        cx.bot_tx
            .send(BotCommand::respond(
                &msg,
                format!("parting {channels}"),
                false,
            ))
            .await
            .unwrap();
        for chan in args {
            cx.bot_tx
                .send(BotCommand::PartChannel(chan.to_owned()))
                .await
                .unwrap();
        }
    }
}

async fn join(cx: CommandContext<BotCommand>) {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return;
    };

    let args = msg
        .message_text()
        .split_ascii_whitespace()
        .skip(1)
        .collect::<Vec<_>>();

    log::info!("Joining {}", args.join(", "));

    cx.bot_tx
        .send(BotCommand::respond(
            &msg,
            format!("joining {}", args.join(", ")),
            false,
        ))
        .await
        .unwrap();

    for chan in args {
        cx.bot_tx
            .send(BotCommand::JoinChannel(chan.into()))
            .await
            .unwrap();
    }
}
