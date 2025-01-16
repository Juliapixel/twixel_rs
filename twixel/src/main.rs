use bot::{Bot, BotCommand};
use cli::ARGS;
use command::{Command, CommandGuard, CommandHandler};
use guard::{Guard, UserGuard};

mod bot;
mod cli;
mod command;
mod guard;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "INFO"
        },
    ));

    log::info!("twixel bot started");

    let bot = Bot::new(
        dotenvy::var("TWITCH_LOGIN").unwrap(),
        dotenvy::var("TWITCH_TOKEN").unwrap(),
    )
    .await
    .add_channels(&ARGS.channels)
    .await
    .add_command(Command::new(
        HandlerGeneric,
        vec!["test".into(), "testing".into()],
        "%",
    ))
    .add_command(Command::new_with_guard(
        HandlerGeneric,
        UserGuard::allow("173685614").and(CommandGuard::new(vec!["elevated".into()], "%")),
    ));

    bot.run().await;
    Ok(())
}

struct HandlerGeneric;

impl CommandHandler for HandlerGeneric {
    fn handle(
        &self,
        cx: command::CommandContext<BotCommand>,
    ) -> std::boxed::Box<(dyn std::future::Future<Output = ()> + 'static)> {
        Box::new(async move {
            println!("{}", cx.msg.raw());
            cx.bot_tx
                .send(BotCommand::SendMessage {
                    channel_login: cx.msg.get_param(0).unwrap().into(),
                    message: "HIIII".into(),
                    reply_id: None,
                })
                .await
                .unwrap();
        })
    }
}
