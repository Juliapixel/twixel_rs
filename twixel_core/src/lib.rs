#![deny(clippy::missing_safety_doc)]
// utf-8 char boundary checking is cool
#![allow(clippy::sliced_string_as_bytes)]

pub mod auth;
pub mod connection;
pub mod irc_message;
pub mod user;

pub use crate::auth::Auth;
pub use crate::connection::{Connection, ConnectionPool};
pub use crate::irc_message::builder::MessageBuilder;
pub use crate::irc_message::command::IrcCommand;
pub use crate::irc_message::message::IrcMessage;
