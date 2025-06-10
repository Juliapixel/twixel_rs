use sqlx::SqlitePool;
use twixel_core::irc_message::PrivMsg;

use crate::{handler::extract::Data, util::db::upsert_user};

pub async fn test(Data(pool): Data<SqlitePool>, msg: PrivMsg<'static>) -> String {
    let mut conn = pool.acquire().await.unwrap();

    // "unimplemented! :3".into()
    match upsert_user(&mut *conn, &msg).await {
        Ok(_) => "added user! :3".into(),
        Err(e) => e.to_string(),
    }
}
