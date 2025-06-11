use std::str::FromStr;

use bot::Bot;
use cli::ARGS;
use commands::{argtest, bread_fact, cat_fact, join, part, sql, strdbg, suggest, test};
use config::CONFIG;
use guard::UserGuard;
use handler::{Command, CommandBuilder};
use sqlx::sqlite::SqliteConnectOptions;

use crate::handler::response::BotResponse;

mod anymap;
mod bot;
mod cli;
mod commands;
mod config;
mod eval;
mod guard;
mod handler;
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

    let db_url = format!(
        "sqlite://{}",
        CONFIG
            .database
            .path
            // .canonicalize()
            // .expect("failed to canonicalize DB path")
            .as_os_str()
            .to_string_lossy(),
    );

    let db = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&db_url)
            .expect("bad sqlite DB url")
            .create_if_missing(true)
            .optimize_on_close(true, None),
    )
    .await
    .expect("failed to create SQLITE DB pool");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let bot = Bot::new(CONFIG.twitch.login.clone(), CONFIG.twitch.token.clone())
        .await
        .add_channels(ARGS.channels.iter().map(|s| s.as_str()))
        .await
        .data(db)
        .add_command(Command::new(async || "hi", vec!["hi".into()], "%"))
        // .catchall(handle_joefish)
        .add_command(
            CommandBuilder::new(join, vec!["join".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        .add_command(
            CommandBuilder::new(sql, vec!["sql".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        // .add_command(
        //     CommandBuilder::new(wrap_fn(remindfish), vec!["remindfish".into()], "%").build(),
        // )
        .add_command(CommandBuilder::new(suggest, vec!["suggest".into()], "%").build())
        .add_command(
            CommandBuilder::new(part, vec!["part".into(), "leave".into()], "%")
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
            CommandBuilder::new(strdbg, vec!["strdbg".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        .add_command(Command::new(
            async || "idk bro figure it out",
            vec!["help".into(), "commands".into()],
            "%",
        ))
        .add_command(Command::new(
            async || "that's my job! >:ε",
            vec!["pong".into()],
            "%",
        ))
        .add_command(Command::new(async || "pong! :3c", vec!["ping".into()], "%"))
        .add_command(Command::new(
            async || "shes hot and funny and smart and pretty and everyone likes her!",
            vec!["juliafact".into()],
            "%",
        ))
        .add_command(Command::new(async || "🪑", vec!["tucfact".into()], "%"))
        .add_command(Command::new(cat_fact, vec!["catfact".into()], "%"))
        .add_command(Command::new(bread_fact, vec!["breadfact".into()], "%"))
        .add_command(Command::new(argtest, vec!["argtest".into()], "%"))
        .add_command(Command::new(test, vec!["test".into()], "%"))
        .add_command(
            CommandBuilder::new(
                async || ("shutting down!", BotResponse::Shutdown),
                vec!["strdbg".into()],
                "%",
            )
            .and(UserGuard::allow([JULIA_ID]))
            .build(),
        );

    log::info!("twixel bot started");

    bot.run().await;
    Ok(())
}
