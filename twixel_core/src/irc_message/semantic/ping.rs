use crate::MessageBuilder;

use super::Ping;

impl Ping<'_> {
    pub fn respond(&self) -> MessageBuilder<'_> {
        MessageBuilder::pong(self.get_param(0).unwrap())
    }
}
