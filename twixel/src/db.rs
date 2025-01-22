use sqlx::{Acquire, Executor};
use twixel_core::irc_message::{tags::OwnedTag, PrivMsg};

pub struct TwitchUser {
    twitch_id: String,
    twitch_login: String,
    twitch_display_name: String,
}

pub async fn get_user_by_twitch_id(
    executor: impl Executor<'_, Database = sqlx::Sqlite>,
    id: &str,
) -> Result<Option<TwitchUser>, sqlx::Error> {
    sqlx::query_as!(
        TwitchUser,
        "
        SELECT
            twitch_id,
            twitch_login,
            twitch_display_name
        FROM twitch_users
        WHERE twitch_id = ?1;
        ",
        id
    )
    .fetch_optional(executor)
    .await
}

pub async fn upsert_user(
    executor: impl Acquire<'_, Database = sqlx::Sqlite>,
    msg: &PrivMsg<'_>,
) -> Result<(), sqlx::Error> {
    let Some(user_login) = msg.sender_login() else {
        return Ok(());
    };
    let Some(user_id) = msg.sender_id() else {
        return Ok(());
    };
    let Some(user_display_name) = msg.get_tag(OwnedTag::DisplayName) else {
        return Ok(());
    };

    let mut trans = executor.begin().await?;

    if get_user_by_twitch_id(&mut *trans, user_id).await?.is_some() {
        sqlx::query!(
            "
            UPDATE twitch_users SET
            twitch_login = ?1,
            twitch_display_name = ?2
            WHERE twitch_id = ?3
            ",
            user_login,
            user_display_name,
            user_id
        )
        .execute(&mut *trans)
        .await?;
    } else {
        let now = chrono::Utc::now();
        let local_user_id = sqlx::query_scalar!(
            "
            INSERT INTO users (
                creation_ts
            )
            VALUES
            (?1)
            RETURNING id
            ",
            now
        )
        .fetch_one(&mut *trans)
        .await?;
        sqlx::query!(
            "
            INSERT INTO twitch_users (
                user_id,
                twitch_id,
                twitch_login,
                twitch_display_name
            )
            VALUES
            (?1, ?2, ?3, ?4)
            ",
            local_user_id,
            user_id,
            user_login,
            user_display_name
        )
        .execute(&mut *trans)
        .await?;
    }
    trans.commit().await
}
