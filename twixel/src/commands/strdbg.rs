use either::Either;
use twixel_core::irc_message::{PrivMsg, Whisper};
use unicode_segmentation::UnicodeSegmentation;

use crate::{bot::BotCommand, command::CommandContext};

pub async fn strdbg(cx: CommandContext<Either<PrivMsg<'static>, Whisper<'static>>>) {
    let Either::Left(msg) = cx.msg else {
        return;
    };

    let response = format!(
        "{} graphemes, {} chars, {} bytes, {:?}",
        msg.message_text().graphemes(true).count(),
        msg.message_text().chars().count(),
        msg.message_text().len(),
        msg.message_text()
    );

    cx.bot_tx
        .send(BotCommand::respond(&msg, response, false))
        .await
        .unwrap()
}
