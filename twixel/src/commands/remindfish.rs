use std::{sync::LazyLock, time::Duration};

use either::Either;
use sqlx::SqlitePool;
use twixel_core::irc_message::{AnySemantic, PrivMsg, Whisper};

use crate::{
    bot::BotCommand,
    command::{extract::Data, CommandContext},
    util::db::{get_user_by_twitch_id, upsert_user},
};

static CATCH_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"@[a-zA-Z0-9_]+, You caught a [✨🫧] {1,2}(\S+) {1,2}[✨🫧]").unwrap()
});

fn write_random_spice_text(mut w: impl core::fmt::Write,catch: &str) -> core::fmt::Result {
    match rand::random_range(0..5) {
        0 => write!(w, "that was a really nice {catch} earlier but now you gotta lock in!"),
        1 => write!(w, "wake up lazybones, there's fish to catch!"),
        2 => write!(w, "its fishin' time."),
        3 => write!(w, "women want you, fish fear you etc. etc. now go fish!"),
        4 => write!(w, "you dont get paid to laze around all day, go fish!."),
        _ => write!(w, "go fish!")
    }
}

pub async fn handle_joefish(cx: CommandContext<AnySemantic<'static>>) -> bool {
    let AnySemantic::PrivMsg(msg) = cx.msg else {
        return false;
    };

    if msg.sender_id().is_none_or(|id| id != "951349582") {
        return false;
    }

    let Some(catch) = CATCH_REGEX
        .captures(msg.message_text())
        .and_then(|c| c.get(1))
        .map(|c| c.as_str().to_owned())
        else
    {
        return true;
    };
    log::info!("new catch! {catch:?}");

    let mut conn = cx
        .data_store
        .get::<SqlitePool>()
        .unwrap()
        .acquire()
        .await
        .unwrap();

    if let Ok(Some(user)) = get_user_by_twitch_id(&mut *conn, msg.sender_id().unwrap()).await
        && user.fish_reminder()
    {
        tokio::spawn(async move {
            log::info!("fish reminder set for {}", msg.sender_login().unwrap());
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
            let mut reminder = format!(
                "@{}, ",
                msg.sender_login().unwrap()
            );
            write_random_spice_text(&mut reminder, catch.as_str()).unwrap();
            if let Err(e) = cx
                .bot_tx
                .send(BotCommand::respond(
                    &msg,
                    reminder,
                    false,
                ))
                .await
            {
                log::error!("Failed to remind user to fish! {e}");
            }
        });
    } else {
        log::debug!("fish caught but no reminder set for id {}", msg.sender_id().unwrap())
    }

    false
}

pub async fn remindfish(Data(pool): Data<SqlitePool>) -> &'static str {
    let mut conn = pool
        .acquire()
        .await
        .unwrap();

    let user = match upsert_user(&mut *conn, &msg).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            log::error!("Failed to upsert user.");
            return;
        }
        Err(e) => {
            cx.bot_tx
                .send(BotCommand::respond(&msg, e.to_string(), false))
                .await
                .unwrap();
            return;
        }
    };

    let uid = user.id();

    match sqlx::query!(
        "UPDATE users SET
            fish_reminder = 1 - fish_reminder
            WHERE
            id = ?1
            RETURNING fish_reminder",
        uid
    )
    .fetch_one(&mut *conn)
    .await
    {
        Ok(r) => {
            let reply = if r.fish_reminder == 1 {
                "reminder created!"
            } else {
                "reminder removed!"
            };
            cx.bot_tx
                .send(BotCommand::respond(&msg, reply.into(), false))
                .await
                .unwrap()
        }
        Err(e)
            if e.as_database_error()
                .is_some_and(|e| e.is_unique_violation()) =>
        {
            cx.bot_tx
                .send(BotCommand::respond(
                    &msg,
                    "ur already reminded duh".into(),
                    false,
                ))
                .await
                .unwrap()
        }
        Err(e) => {
            log::error!("Failed to create fish reminder: {e}");
            cx.bot_tx
                .send(BotCommand::respond(
                    &msg,
                    "Failed to create fish reminder".into(),
                    false,
                ))
                .await
                .unwrap()
        }
    }
}
