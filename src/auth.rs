use rand::Rng;

#[derive(Debug, Default, Clone)]
pub enum Auth {
    OAuth{ username: String, token: String },
    #[default]
    Anonymous
}

impl Auth {
    pub fn into_commands(&self) -> (String, String) {
        match self {
            Self::OAuth { username, token } => {
                (
                    String::from(format!("NICK {username}")),
                    String::from(format!("PASS {token}")),
                )
            },
            Self::Anonymous => {
                let mut rng = rand::thread_rng();
                (
                    String::from(format!("NICK justinfan{}", rng.gen_range(1..99999))),
                    String::from("PASS POGGERS")
                )
            }
        }
    }
}
