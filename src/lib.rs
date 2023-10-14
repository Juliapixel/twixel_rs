#![allow(dead_code)]

pub mod client_builder;
pub(crate) mod irc_message;
pub(crate) mod connection;
pub(crate) mod user;
pub(crate) mod auth;

pub use self::client_builder::{ClientBuilder, TwitchIRCClient};
