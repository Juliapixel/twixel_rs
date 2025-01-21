use either::Either;
use twixel_core::irc_message::{PrivMsg, Whisper};

use crate::{bot::BotCommand, command::CommandContext};

pub async fn part(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let args = msg
        .message_text()
        .split_ascii_whitespace()
        .skip(1)
        .collect::<Vec<_>>();

    let source_channel = msg.channel_login();

    if args.is_empty() {
        let source_channel = source_channel.to_owned();
        cx.bot_tx
            .send(BotCommand::respond(&msg, "byeeee :333".into(), false))
            .await
            .unwrap();
        cx.bot_tx
            .send(BotCommand::PartChannel(source_channel))
            .await
            .unwrap();
    } else {
        let channels = args.join(", ");

        cx.bot_tx
            .send(BotCommand::respond(
                &msg,
                format!("parting {channels}"),
                false,
            ))
            .await
            .unwrap();
        for chan in args {
            cx.bot_tx
                .send(BotCommand::PartChannel(chan.to_owned()))
                .await
                .unwrap();
        }
    }
}
