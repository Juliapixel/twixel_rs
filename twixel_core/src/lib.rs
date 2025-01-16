// #![allow(dead_code)]

// pub mod client_builder;
pub mod auth;
pub mod connection;
pub mod irc_message;
pub mod user;

pub use crate::auth::Auth;
pub use crate::connection::{Connection, ConnectionPool};
pub use crate::irc_message::builder::MessageBuilder;
pub use crate::irc_message::command::IrcCommand;
pub use crate::irc_message::message::IrcMessage;

// pub use self::client_builder::{ClientBuilder, TwitchIRCClient};
