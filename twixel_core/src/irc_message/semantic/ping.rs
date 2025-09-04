use crate::MessageBuilder;

use super::Ping;

impl Ping {
    pub fn respond(&'_ self) -> MessageBuilder<'_> {
        MessageBuilder::pong(self.get_param(0).unwrap())
    }
}
