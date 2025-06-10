use crate::handler::{
    extract::{Channel, MessageText},
    response::BotResponse,
};

pub async fn part(
    MessageText(msg): MessageText,
    Channel(source_chan): Channel,
) -> (String, Vec<BotResponse>) {
    let args = msg.split_ascii_whitespace().skip(1).collect::<Vec<_>>();

    if args.is_empty() {
        ("byeeee :333".into(), vec![BotResponse::Part(source_chan)])
    } else {
        let channels = args.join(", ");

        (
            format!("parting {channels}"),
            args.iter()
                .map(|c| BotResponse::Part(c.to_string()))
                .collect(),
        )
    }
}
