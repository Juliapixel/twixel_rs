use reqwest::Client;
use serde::Deserialize;
use twixel_core::irc_message::PrivMsg;

#[derive(Debug, Deserialize)]
struct HasteResp {
    key: String
}

pub async fn raw(msg: PrivMsg) -> String {

    let resp = Client::new().post("https://haste.potat.app/documents")
        .body(reqwest::Body::from(serde_json::to_string_pretty(&*msg).unwrap()))
        .send()
        .await;
    let Ok(r) = resp else {
        return "Error: couldnt upload to hastebin lole".into();
    };
    let Ok(key) = r.json::<HasteResp>().await.map(|b| b.key) else {
        return "Error: couldnt deserialize hastebin response lole".into();
    };
    format!("https://haste.potat.app/{key}")
}
