use std::{sync::LazyLock, time::Duration};

use sqlx::SqlitePool;

use crate::{
    handler::{
        extract::{Data, MessageText, SenderId},
        response::{BotResponse, DelayedResponse, IntoResponse},
    },
    util::db::{TwixelUser, get_user_by_twitch_login},
};

static CATCH_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"@([a-zA-Z0-9_]+), You caught a [✨🫧] {1,2}(\S+) {1,2}[✨🫧]").unwrap()
});

fn write_random_spice_text(mut w: impl core::fmt::Write, catch: &str) -> core::fmt::Result {
    match rand::random_range(0..5) {
        0 => write!(
            w,
            "that was a really nice {catch} earlier but now you gotta lock in!"
        ),
        1 => write!(w, "wake up lazybones, there's fish to catch!"),
        2 => write!(w, "its fishin' time."),
        3 => write!(w, "women want you, fish fear you etc. etc. now go fish!"),
        4 => write!(w, "you dont get paid to laze around all day, go fish!."),
        _ => write!(w, "go fish!"),
    }
}

const GOFISHGAME_ID: &str = "951349582";

pub async fn handle_joefish(
    MessageText(text): MessageText,
    SenderId(id): SenderId,
    Data(pool): Data<SqlitePool>,
) -> Option<DelayedResponse<BotResponse>> {
    if id != GOFISHGAME_ID {
        return None;
    }
    let (Some(fisherman), Some(catch)) =
        CATCH_REGEX.captures(&text).map(|c| (c.get(1), c.get(2)))?
    else {
        return None;
    };
    let (fisherman, catch) = (fisherman.as_str(), catch.as_str());
    log::info!("new catch! {catch:?}");

    let mut conn = pool.acquire().await.unwrap();

    let user = match get_user_by_twitch_login(&mut *conn, fisherman).await {
        Ok(u) => u?,
        Err(e) => {
            return Some(DelayedResponse(
                e.into_response().await?,
                Duration::new(0, 0),
            ));
        }
    };
    if user.fish_reminder() {
        log::info!("fish reminder set for {fisherman}");
        let mut reminder = format!("@{fisherman}, ");
        write_random_spice_text(&mut reminder, catch).unwrap();
        Some(DelayedResponse(
            reminder.into_response().await?,
            Duration::from_secs(30 * 60),
        ))
    } else {
        log::debug!("fish caught but no reminder set for {fisherman}");
        None
    }
}

pub async fn remindfish(user: TwixelUser, Data(pool): Data<SqlitePool>) -> &'static str {
    let mut conn = pool.acquire().await.unwrap();

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
            if r.fish_reminder == 1 {
                "reminder created!"
            } else {
                "reminder removed!"
            }
        }
        Err(e) => {
            log::error!("Failed to create fish reminder: {e}");
            "Failed to create fish reminder"
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{commands::handle_joefish, handler::assert_is_handler};

    #[test]
    fn wah() {
        assert_is_handler(handle_joefish);
    }

    #[test]
    fn catch_regex() {
        assert!(
            super::CATCH_REGEX
                .captures("@gawblemachine, You caught a ✨ 🪝 ✨ ! It weighs 1.41 lbs. (30m cooldown after a catch)")
                .is_some_and(|c| c.get(1).is_some_and(|c| c.as_str() == "🪝"))
        )
    }
}
