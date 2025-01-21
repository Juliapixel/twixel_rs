use either::Either;
use sqlx::{sqlite::SqliteRow, Column, Executor, Row, SqlitePool};
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext};

fn to_json(rows: &[SqliteRow]) -> serde_json::Value {
    let mut out = vec![];

    for row in rows {
        let mut map = serde_json::Map::new();

        for i in 0..row.len() {
            match row.try_get::<Option<&str>, _>(i) {
                Ok(Some(s)) => {
                    map.insert(
                        row.column(i).name().to_owned(),
                        serde_json::Value::String(s.to_owned()),
                    );
                    continue;
                }
                Ok(None) => {
                    map.insert(row.column(i).name().to_owned(), serde_json::Value::Null);
                    continue;
                }
                Err(_) => (),
            };
            match row.try_get::<Option<f64>, _>(i) {
                Ok(Some(o)) => {
                    map.insert(
                        row.column(i).name().to_owned(),
                        serde_json::Value::Number(serde_json::Number::from_f64(o).unwrap()),
                    );
                    continue;
                }
                Ok(None) => {
                    map.insert(row.column(i).name().to_owned(), serde_json::Value::Null);
                    continue;
                }
                Err(_) => (),
            };
            match row.try_get::<Option<i64>, _>(i) {
                Ok(Some(o)) => {
                    map.insert(
                        row.column(i).name().to_owned(),
                        serde_json::Value::Number(o.into()),
                    );
                    continue;
                }
                Ok(None) => {
                    map.insert(row.column(i).name().to_owned(), serde_json::Value::Null);
                    continue;
                }
                Err(_) => (),
            };
            match row.try_get::<Option<Box<[u8]>>, _>(i) {
                Ok(Some(o)) => {
                    map.insert(
                        row.column(i).name().to_owned(),
                        serde_json::Value::Array(
                            o.iter()
                                .map(|n| serde_json::Value::Number((*n).into()))
                                .collect(),
                        ),
                    );
                    continue;
                }
                Ok(None) => {
                    map.insert(row.column(i).name().to_owned(), serde_json::Value::Null);
                    continue;
                }
                Err(_) => (),
            };
        }

        out.push(serde_json::Value::Object(map));
    }

    serde_json::Value::Array(out)
}

pub async fn sql(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else { return };

    let Some((_cmd, query)) = msg.message_text().split_once(' ') else {
        cx.bot_tx
            .send(BotCommand::respond(
                &msg,
                "bro send a query are u dumb".into(),
                false,
            ))
            .await
            .unwrap();
        return;
    };

    let pool = cx.data_store.get::<SqlitePool>().unwrap();

    let mut conn = pool.acquire().await.unwrap();

    let start = std::time::Instant::now();
    let query = query.to_owned();

    let res = conn.fetch_all(query.as_str()).await;

    let elapsed = start.elapsed();

    match res {
        Ok(r) => {
            let row_count = r.len();

            let out = to_json(&r);

            let response = format!(
                "{} ROWS AFFECTED, took {}ms; {out}",
                row_count,
                elapsed.as_millis()
            );
            cx.bot_tx
                .send(BotCommand::respond(&msg, response, false))
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
