use sqlx::{Executor, SqlitePool};

use crate::handler::extract::{Data, MessageText, SenderId};

pub async fn suggest(
    MessageText(msg): MessageText,
    SenderId(sender_id): SenderId,
    Data(pool): Data<SqlitePool>,
) -> Option<String> {
    let suggestion = msg.split_once(' ').map(|s| s.1)?;

    let mut conn = pool.acquire().await.unwrap();

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
        Ok(_) => Some("suggestion saved successfully!".into()),
        Err(err) => {
            log::error!("{err}");
            Some(err.to_string())
        }
    }
}
