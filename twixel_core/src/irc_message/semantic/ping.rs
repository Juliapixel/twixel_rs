use crate::MessageBuilder;

use super::Ping;

impl Ping {
    /// Creates a new [MessageBuilder](crate::MessageBuilder) containing a PONG
    /// for this PING
    pub fn respond(&'_ self) -> MessageBuilder<'_> {
        MessageBuilder::pong(self.get_param(0).unwrap_or_default())
    }
}
