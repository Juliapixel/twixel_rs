use either::Either;
use futures::TryFutureExt;
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext};

#[derive(serde::Deserialize)]
struct CatFact {
    fact: String,
}

pub async fn cat_fact(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let resp = match reqwest::get("https://catfact.ninja/fact")
        .and_then(|r| r.json::<CatFact>())
        .await
    {
        Ok(f) => f.fact,
        Err(e) => e.to_string(),
    };

    cx.bot_tx
        .send(BotCommand::respond(&msg, resp, false))
        .await
        .unwrap();
}
