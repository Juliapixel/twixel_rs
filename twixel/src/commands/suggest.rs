use either::Either;
use sqlx::{Executor, SqlitePool};
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext};

pub async fn suggest(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let Some(suggestion) = msg.message_text().split_once(' ').map(|s| s.1) else {
        return;
    };

    let Some(sender_id) = msg.sender_id() else {
        log::error!("no sender id in suggest command");
        return;
    };

    let mut conn = cx
        .data_store
        .get::<SqlitePool>()
        .unwrap()
        .acquire()
        .await
        .unwrap();

    let query = sqlx::query(
        "
        INSERT INTO suggestions
        (suggestion, sender_id)
        VALUES
        (?1, ?2);
        ",
    )
    .bind(suggestion)
    .bind(sender_id);

    match conn.execute(query).await {
        Ok(_) => {
            cx.bot_tx
                .send(BotCommand::respond(
                    &msg,
                    "suggestion saved successfully!".into(),
                    false,
                ))
                .await
                .unwrap();
        }
        Err(err) => {
            log::error!("{err}");
            cx.bot_tx
                .send(BotCommand::respond(&msg, err.to_string(), false))
                .await
                .unwrap()
        }
    }
}
