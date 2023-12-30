use rand::Rng;

use crate::irc_message::{owned::OwnedIrcMessage, command::IrcCommand};

#[derive(Debug, Default, Clone)]
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
                        params: vec![String::from(format!("PASS {token}"))],
                    },
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Nick,
                        params: vec![String::from(format!("NICK {username}"))],
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
                        params: vec![String::from("PASS POGGERS")],
                    },
                    OwnedIrcMessage {
                        tags: None,
                        prefix: None,
                        command: IrcCommand::Nick,
                        params: vec![String::from(format!("NICK justinfan{}", rng.gen_range(1..99999)))],
                    },
                )
            }
        }
    }
}
