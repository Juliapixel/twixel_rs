use std::fmt::Debug;

use rand::Rng;

use crate::irc_message::{owned::OwnedIrcMessage, command::IrcCommand};

#[derive(Default, Clone)]
pub enum Auth {
    OAuth{ username: String, token: String },
    #[default]
    Anonymous
}

impl Auth {
    pub fn into_commands(&self) -> (OwnedIrcMessage, OwnedIrcMessage) {
        match self {
            Self::OAuth { username, token } => {
                (
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Pass,
                        params: vec![String::from(format!("{token}"))],
                    },
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Nick,
                        params: vec![String::from(format!("{username}"))],
                    },
                )
            },
            Self::Anonymous => {
                let mut rng = rand::thread_rng();
                (
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Pass,
                        params: vec![String::from("POGGERS")],
                    },
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Nick,
                        params: vec![String::from(format!("justinfan{}", rng.gen_range(1..99999)))],
                    },
                )
            }
        }
    }
}

impl Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OAuth { username, token: _ } => f.debug_struct("OAuth").field("username", username).field("token", &"*redacted for privacy*").finish(),
            Self::Anonymous => write!(f, "Anonymous"),
        }
    }
}