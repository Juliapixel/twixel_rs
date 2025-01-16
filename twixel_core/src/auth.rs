use std::fmt::Debug;

use rand::Rng;

use crate::irc_message::{builder::MessageBuilder, command::IrcCommand};

#[derive(Default, Clone)]
pub enum Auth {
    OAuth {
        username: String,
        token: String,
    },
    #[default]
    Anonymous,
}

impl Auth {
    pub fn into_commands(&self) -> (MessageBuilder<'_>, MessageBuilder<'_>) {
        match self {
            Self::OAuth { username, token } => (
                MessageBuilder::new(IrcCommand::Pass).add_param(format!("oauth:{token}")),
                MessageBuilder::new(IrcCommand::Nick).add_param(username.as_str()),
            ),
            Self::Anonymous => {
                let mut rng = rand::thread_rng();
                (
                    MessageBuilder::new(IrcCommand::Pass).add_param("POGGERS"),
                    MessageBuilder::new(IrcCommand::Nick)
                        .add_param(format!("justinfan{}", rng.gen_range(1..99999))),
                )
            }
        }
    }
}

impl Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OAuth { username, token: _ } => f
                .debug_struct("OAuth")
                .field("username", username)
                .field("token", &"*redacted for privacy*")
                .finish(),
            Self::Anonymous => write!(f, "Anonymous"),
        }
    }
}
