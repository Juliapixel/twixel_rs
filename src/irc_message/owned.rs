use std::fmt::Display;

use super::{command::IrcCommand, tags::OwnedTag, prefix::OwnedPrefix};

#[derive(Debug)]
pub struct OwnedIrcMessage {
    pub tags: Option<Vec<(OwnedTag, String)>>,
    pub prefix: Option<OwnedPrefix>,
    pub command: IrcCommand,
    pub params: Vec<String>
}

impl Display for OwnedIrcMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tags) = &self.tags {
            write!(f, "@")?;
            let mut first = true;
            for i in tags {
                write!(f, "{}{}={}", if first { "" } else { ";" }, Into::<&str>::into(&i.0), i.1)?;
                first = false;
            }
            write!(f, " ")?;
        }
        if let Some(prefix) = &self.prefix {
            write!(f, "{} ", prefix.to_string())?;
        }
        write!(f, "{}", Into::<&str>::into(self.command))?;

        for (i, val) in self.params.iter().enumerate() {
            write!(
                f,
                " {}{}",
                val,
                if i == self.params.len() - 1 { "\r\n" } else { "" }
            )?;
        }

        return Ok(());
    }
}
