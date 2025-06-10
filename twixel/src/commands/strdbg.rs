use unicode_segmentation::UnicodeSegmentation;

use crate::handler::extract::MessageText;

pub async fn strdbg(MessageText(msg): MessageText) -> String {
    format!(
        "{} graphemes, {} chars, {} bytes, {:?}",
        msg.graphemes(true).count(),
        msg.chars().count(),
        msg.len(),
        msg
    )
}
