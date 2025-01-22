use std::str::FromStr;

use bot::Bot;
use cli::ARGS;
use command::{wrap_fn, Command, CommandBuilder, StaticMessageHandler};
use commands::{cat_fact, join, part, sql, strdbg, suggest, test};
use config::CONFIG;
use guard::UserGuard;
use sqlx::sqlite::SqliteConnectOptions;

mod anymap;
mod bot;
mod cli;
mod command;
mod commands;
mod config;
mod db;
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
        .add_command(
            CommandBuilder::new(wrap_fn(join), vec!["join".into()], "%")
                .and(UserGuard::allow([JULIA_ID]))
                .build(),
        )
        .add_command(CommandBuilder::new(wrap_fn(sql), vec!["sql".into()], "%").build())
        .add_command(CommandBuilder::new(wrap_fn(test), vec!["test".into()], "%").build())
        .add_command(CommandBuilder::new(wrap_fn(suggest), vec!["suggest".into()], "%").build())
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
