use crate::handler::{
    extract::MessageText,
    response::{BotResponse, IntoResponse},
};

pub async fn join(MessageText(msg): MessageText) -> impl IntoResponse {
    let args = msg.split_ascii_whitespace().skip(1).collect::<Vec<_>>();

    log::info!("Joining {}", args.join(", "));

    let joins: Vec<BotResponse> = args
        .iter()
        .map(|arg| BotResponse::Join(arg.to_string()))
        .collect();

    (format!("joining {}", args.join(", ")), joins)
}
