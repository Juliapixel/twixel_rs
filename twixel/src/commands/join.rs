use either::Either;
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext};

pub async fn join(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let args = msg
        .message_text()
        .split_ascii_whitespace()
        .skip(1)
        .collect::<Vec<_>>();

    log::info!("Joining {}", args.join(", "));

    cx.bot_tx
        .send(BotCommand::respond(
            &msg,
            format!("joining {}", args.join(", ")),
            false,
        ))
        .await
        .unwrap();

    for chan in args {
        cx.bot_tx
            .send(BotCommand::JoinChannel(chan.into()))
            .await
            .unwrap();
    }
}
