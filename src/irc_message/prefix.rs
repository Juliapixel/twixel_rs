#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Prefix {
    OnlyHostname{ host: String },
    Full{ nickname: String, username: String, host: String }
}

impl From<&str> for Prefix {
    fn from(value: &str) -> Self {
        match value.split_once('@') {
            Some(splits) => {
                let (nickname, username) = splits.0.split_once('!').unwrap();
                let hostname = splits.1.to_string();
                Self::Full {
                    nickname: String::from(nickname),
                    username: String::from(username),
                    host: hostname
                }
            },
            None => Self::OnlyHostname{ host: value.to_string() }
        }
    }
}
