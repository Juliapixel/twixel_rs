use sqlx::{Column, Executor, Row, SqlitePool, sqlite::SqliteRow};

use crate::handler::extract::{Data, MessageText};

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

pub async fn sql(MessageText(msg): MessageText, Data(pool): Data<SqlitePool>) -> String {
    let Some((_cmd, query)) = msg.split_once(' ') else {
        return "bro send a query are u dumb".into();
    };

    let mut conn = pool.acquire().await.unwrap();

    let start = std::time::Instant::now();
    let query = query.to_owned();

    let res = conn.fetch_all(query.as_str()).await;

    let elapsed = start.elapsed();

    match res {
        Ok(r) => {
            let row_count = r.len();

            let out = to_json(&r);

            format!(
                "{} ROWS AFFECTED, took {}ms; {out}",
                row_count,
                elapsed.as_millis()
            )
        }
        Err(e) => e.to_string(),
    }
}
