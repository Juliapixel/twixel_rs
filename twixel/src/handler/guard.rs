use twixel_core::irc_message::AnySemantic;

use crate::guard::{Guard, GuardContext};

#[derive(Clone)]
pub struct CommandGuard {
    names: Vec<String>,
    prefix: String,
}

impl CommandGuard {
    pub fn new(names: Vec<String>, prefix: impl Into<String>) -> Self {
        Self {
            names,
            prefix: prefix.into(),
        }
    }
}

impl Guard for CommandGuard {
    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(self.clone())
    }

    fn check(&self, ctx: &GuardContext) -> bool {
        if let AnySemantic::PrivMsg(msg) = ctx.message {
            let text = msg.message_text();
            let Some(first_word) = text.split_ascii_whitespace().next() else {
                return false;
            };
            let Some((prefix, cmd)) = first_word.split_at_checked(1) else {
                return false;
            };
            prefix == self.prefix && self.names.iter().any(|name| name == cmd)
        } else {
            false
        }
    }
}
