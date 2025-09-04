//! Trait and implementations for auth methods for IRC

use std::fmt::Debug;

use rand::Rng;

use crate::irc_message::{builder::MessageBuilder, command::IrcCommand};

/// Trait for IRC auth providers
pub trait AuthProvider {
    /// Returns a tuple where the first item is the first param to a
    /// [`PASS`](crate::irc_message::Pass) message and the second item is the
    /// first param to a [`NICK`](crate::irc_message::Nick) message
    fn pass_nick(&mut self) -> (String, String);

    /// Provided method that returns a tuple of a [`PASS`](crate::irc_message::Pass)
    /// and a [`NICK`](crate::irc_message::Nick) message, to be sent to the IRC
    /// server
    fn get_commands(&mut self) -> (MessageBuilder<'_>, MessageBuilder<'_>) {
        let (pass, nick) = self.pass_nick();
        (
            MessageBuilder::new(IrcCommand::Pass).add_param(pass),
            MessageBuilder::new(IrcCommand::Nick).add_param(nick),
        )
    }
}

/// Anonymous login auth implementation
#[derive(Debug, Clone, Copy)]
pub struct Anonymous;

impl AuthProvider for Anonymous {
    fn pass_nick(&mut self) -> (String, String) {
        ("POGGERS".into(), format!("justinfan{}", rand::rng().random_range(1..99999)))
    }
}

/// Basic OAuth static auth
#[derive(Clone)]
pub struct OAuth {
    /// The OAuth token
    pub oauth: String,
    /// The associated account's login
    pub nick: String,
}

impl AuthProvider for OAuth {
    fn pass_nick(&mut self) -> (String, String) {
        (format!("oauth:{}", self.oauth), self.nick.clone())
    }
}

impl Debug for OAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Standard")
            .field("oauth", &"[REDACTED]")
            .field("nick", &self.nick)
            .finish()
    }
}
