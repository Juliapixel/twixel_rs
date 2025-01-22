use either::Either;
use sqlx::SqlitePool;
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext, db::upsert_user};

pub async fn test(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let mut conn = cx
        .data_store
        .get::<SqlitePool>()
        .unwrap()
        .acquire()
        .await
        .unwrap();

    match upsert_user(&mut *conn, &msg).await {
        Ok(_) => {
            cx.bot_tx
                .send(BotCommand::respond(&msg, "added user! :3".into(), false))
                .await
                .unwrap();
        }
        Err(e) => {
            cx.bot_tx
                .send(BotCommand::respond(&msg, e.to_string(), false))
                .await
                .unwrap();
        }
    }
}
