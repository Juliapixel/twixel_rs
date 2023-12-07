use hashbrown::HashMap;
use thiserror::Error;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "chrono")]
use chrono::{Utc, DateTime};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct IrcTags {
    pub tags: HashMap<String, String>
}

#[derive(Debug, Error)]
pub enum IRCTagParseError {
    #[error("failed to parse the tag due to invalid structure: {0}")]
    TagStructureParseError(String),
    #[error("failed to parse the tag due to unknown error: {0}")]
    ContentParseFailed(String),
}

impl IrcTags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_tags(tags: &[(&str, &str)]) -> Self {
        let mut hashmap: HashMap<String, String> = HashMap::with_capacity(tags.len());
        for (k, v) in tags {
            hashmap.insert(k.to_string(), v.to_string());
        }
        Self {
            tags: hashmap,
        }
    }

    pub fn add_from_string(&mut self, input: &str) {
        if input.find('@') == Some(0) {
            self.add_tags(&input[1..]);
        } else {
            self.add_tags(input);
        }
    }

    fn add_tags(&mut self, input: &str) {
        if input.is_empty() {
            return;
        }
        let tags = input.split(";");
        for tag in tags {
            if let Some((id, value)) = tag.split_once('=') {
                self.tags.insert(id.to_string(), value.to_string());
            } else {
                self.tags.insert(tag.to_string(), String::new());
            }
        }
    }

    pub fn add_single_tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn get_value(&self, query: &str) -> Option<&str> {
        self.tags.get(&query.to_string()).map(|s| s.as_str())
    }

    pub fn get_color(&self) -> Option<[u8; 3]> {
        let char_to_int = |input :char| -> u8 {
            input.to_digit(16).unwrap() as u8
        };

        let val = self.get_value("color")?;
        if val.is_empty() {
            return None
        }
        let individuals: Vec<char> = val.chars().collect();
        return Some([
            char_to_int(individuals[1]) * 16 + char_to_int(individuals[2]),
            char_to_int(individuals[3]) * 16 + char_to_int(individuals[4]),
            char_to_int(individuals[5]) * 16 + char_to_int(individuals[6]),
        ]);
    }

    #[cfg(feature = "chrono")]
    pub fn get_timestamp(&self) -> Option<DateTime<Utc>> {
        let ts = self.get_value("tmi-sent-ts")?.parse::<i64>().ok()?;
        return DateTime::<Utc>::from_timestamp(ts / 1000, 0);
    }

    pub fn get_message_id(&self) -> Option<&str> {
        self.get_value("id")
    }

    pub fn get_sender_id(&self) -> Option<&str> {
        self.get_value("user-id")
    }
}

impl From<IrcTags> for String {
    fn from(tags: IrcTags) -> Self {
        let mut out = String::new();
        if tags.is_empty() {
            return out;
        }
        out += "@";
        for (key, value) in tags.tags {
            out += &key;
            out += "=";
            out += &value;
            out += ";";
        }
        return out;
    }
}

#[derive(Debug, Default)]
pub struct Badge {
    name: String,
    version: i32,
}

#[derive(Debug)]
pub enum SubTier {
    Prime,
    Tier1,
    Tier2,
    Tier3,
}

impl From<&str> for SubTier {
    fn from(value: &str) -> Self {
        match value {
            "1000" => Self::Tier1,
            "2000" => Self::Tier2,
            "3000" => Self::Tier3,
            "Prime" => Self::Prime,
            _ => Self::Tier1,
        }
    }
}
