use serde::Serialize;

use crate::{config::CONFIG, handler::extract::MessageText};

#[derive(Serialize)]
struct ResponsesRequest {
    model: &'static str,
    instructions: &'static str,
    input: String,
}

pub async fn gpt(MessageText(text): MessageText) -> String {
    let Some(api_key) = &CONFIG.openai.api_key else {
        return "SOMEONE forgot to put her API key in the config file".to_string();
    };

    let req = ResponsesRequest {
        model: "gpt-4o-mini",
        instructions: "you are a chatbot in a twitch chat, use the emote \"make\", all lowercase, with spaces on either side, on every message",
        input: text,
    };

    let client = reqwest::Client::new();
    match client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await
    {
        Ok(r) => r.json::<serde_json::Value>().await.unwrap()["output"][0]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        Err(_e) => "request failed!".to_string(),
    }
}
