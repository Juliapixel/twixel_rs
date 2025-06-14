use std::{
    future::{Ready, ready},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use getset::{CopyGetters, Getters};
use sqlx::{Executor, SqlitePool};
use twixel_core::irc_message::{AnySemantic, tags::OwnedTag};

use crate::{
    bot::BotData,
    handler::{
        extract::{Data, Extract},
        response::{BotResponse, IntoResponse},
    },
};

#[derive(Debug, Clone)]
pub struct TwitchUser {
    twitch_id: String,
    twitch_login: String,
    twitch_display_name: String,
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct TwixelUser {
    #[getset(get_copy = "pub")]
    id: i64,
    #[getset(get_copy = "pub")]
    creation_ts: DateTime<Utc>,
    #[getset(get = "pub")]
    role: Option<String>,
    #[getset(get_copy = "pub")]
    fish_reminder: bool,
}

pub async fn get_twitch_user_by_twitch_id(
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

pub async fn get_user_by_twitch_id(
    executor: impl Executor<'_, Database = sqlx::Sqlite>,
    id: &str,
) -> Result<Option<TwixelUser>, sqlx::Error> {
    sqlx::query!(
        "
        SELECT
            u.id,
            u.creation_ts,
            u.role,
            u.fish_reminder
        FROM users AS u FULL OUTER JOIN twitch_users as t ON
        t.user_id = u.id
        WHERE
        t.twitch_id = ?1;
        ",
        id
    )
    .fetch_optional(executor)
    .await
    .map(|o| {
        o.map(|r| TwixelUser {
            id: r.id,
            creation_ts: r.creation_ts.parse().unwrap(),
            role: r.role,
            fish_reminder: r.fish_reminder == 1,
        })
    })
}

pub async fn get_user_by_twitch_login(
    executor: impl Executor<'_, Database = sqlx::Sqlite>,
    login: &str,
) -> Result<Option<TwixelUser>, sqlx::Error> {
    sqlx::query!(
        "
        SELECT
            u.id,
            u.creation_ts,
            u.role,
            u.fish_reminder
        FROM users AS u LEFT JOIN twitch_users as t ON
        t.user_id = u.id
        WHERE
        t.twitch_login = ?1;
        ",
        login
    )
    .fetch_optional(executor)
    .await
    .map(|o| {
        o.map(|r| TwixelUser {
            id: r.id,
            creation_ts: r.creation_ts.parse().unwrap(),
            role: r.role,
            fish_reminder: r.fish_reminder == 1,
        })
    })
}

impl IntoResponse for sqlx::Error {
    fn into_response(self) -> Ready<Option<BotResponse>> {
        log::error!("SQLX error: {self}");
        ready(None)
    }
}

impl Extract for TwixelUser {
    type Error = Option<sqlx::Error>;

    async fn extract(msg: &AnySemantic<'_>, data: Arc<BotData>) -> Result<Self, Self::Error> {
        let pool = Data::<SqlitePool>::extract(msg, data).await.unwrap();

        let AnySemantic::PrivMsg(msg) = msg else {
            return Err(None);
        };

        let Some(user_login) = msg.sender_login() else {
            log::error!("Failed to get sender login from message!");
            return Err(None);
        };
        let Some(user_id) = msg.sender_id() else {
            log::error!("Failed to get sender id from message!");
            return Err(None);
        };
        let Some(user_display_name) = msg.get_tag(OwnedTag::DisplayName) else {
            log::error!("Failed to get display name from message!");
            return Err(None);
        };

        let mut trans = (*pool)
            .begin()
            .await
            .expect("failed to initiate transaction");

        let user = if let Some(user) = get_user_by_twitch_id(&mut *trans, user_id).await? {
            sqlx::query!(
                "
                UPDATE twitch_users SET
                twitch_login = ?1,
                twitch_display_name = ?2
                WHERE twitch_id = ?3
                RETURNING id
                ",
                user_login,
                user_display_name,
                user_id
            )
            .fetch_one(&mut *trans)
            .await?;
            user
        } else {
            let now = chrono::Utc::now();
            let query = sqlx::query!(
                "
                INSERT INTO users (
                    creation_ts
                )
                VALUES
                (?1)
                RETURNING id, creation_ts, role, fish_reminder
                ",
                now
            )
            .fetch_one(&mut *trans)
            .await?;
            let user = TwixelUser {
                id: query.id,
                creation_ts: now,
                role: query.role,
                fish_reminder: query.fish_reminder != 0,
            };
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
                user.id,
                user_id,
                user_login,
                user_display_name
            )
            .execute(&mut *trans)
            .await?;
            user
        };
        trans.commit().await?;

        Ok(user)
    }
}
