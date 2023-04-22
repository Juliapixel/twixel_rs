use std::fmt::Display;
use hashbrown::HashMap;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IRCMessage {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tags: IRCTags,
    pub nick: Option<String>,
    pub command: IRCCommand,
    pub channel: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum IRCMessageParseError {
    TagParseError,
    NoSource,
    NoCommand,
    NoMessage,
    StructureError,
    Empty
}

pub enum IRCMessageFormatter {
    Server,
    Client
}

impl IRCMessage {
    pub fn to_string(self, formatter: IRCMessageFormatter) -> String {
        match formatter {
            IRCMessageFormatter::Server => {
                let mut out = String::new();
                let mut tags: String = self.tags.into();
                let command: String = self.command.into();
                if !tags.is_empty() {
                    tags += " ";
                    out += &tags;
                }
                if let Some(name) = self.nick {
                    out += &format!(":{0}!{0}@{0}.tmi.twitch.tv ", name);
                } else {
                    out += ":tmi.twitch.tv ";
                }
                out += &format!("{} ", command);
                if let Some(channel) = self.channel {
                    out += &format!("#{} ", channel);
                }
                out += ":";
                if let Some(message) = self.message {
                    out += &message;
                }
                return out
            },
            IRCMessageFormatter::Client => {
                let mut out = String::new();
                let mut tags = String::from(self.tags);
                let command: String = self.command.into();
                if !tags.is_empty() {
                    tags += " ";
                    out += &tags;
                }
                out += &format!("{} ", command);
                if let Some(channel) = self.channel {
                    out += &format!("#{} ", channel);
                }
                out += ":";
                if let Some(message) = self.message {
                    out += &message;
                }
                return out
            }
        }
    }

    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.tags.add_single_tag(key, value);
    }

    pub fn get_color(&self) -> Option<[u8; 3]> {
        self.tags.get_color()
    }

    pub fn is_from_mod(&self) -> bool {
        if let Some(value) = self.tags.get_value("mod") {
            return match value {
                "0" => false,
                "1" => true,
                _ => false
            }
        } else {
            return false;
        }
    }

    pub fn get_timestamp_millis(&self) -> Option<u64> {
        if let Some(ts) = self.tags.get_value("tmi-sent-ts") {
            return ts.parse::<u64>().ok();
        } else {
            return None
        }
    }

    pub fn text(message: &str, channel: &str) -> Self {
        Self {
            command: IRCCommand::PrivMsg,
            channel: Some(channel.to_string()),
            nick: None,
            message: Some(message.to_string()),
            tags: IRCTags::default(),
        }
    }
}

// #[allow(non_upper_case_globals)]
// impl From<&str> for IRCMessage {
//     // fn from(value: &str) -> Self {
//     //     lazy_static! {
//     //         static ref privmsg_filter: Regex = regex::RegexBuilder::new(
//     //             r"^(?:@(.+) )?:(?:([a-z0-9_]+)![a-z0-9_]+@[a-z0-9_]+)?\.tmi\.twitch\.tv ([a-zA-Z0-9]+) #(.+) :(.+)"
//     //         ).build().unwrap();
//     //     }
//     //     lazy_static! {
//     //         static ref twitchmsg_filter: Regex = regex::RegexBuilder::new(
//     //             r"^(?:@(.+) )?:tmi\.twitch\.tv ([a-zA-Z0-9]+) #?([a-zA-Z\-_]+)(?: :(.+))?"
//     //         ).build().unwrap();
//     //     }
//     //     lazy_static! {
//     //         static ref miscmsg_filter: Regex = regex::RegexBuilder::new(
//     //             r"([A-Z0-9]+) :tmi\.twitch\.tv"
//     //         ).build().unwrap();
//     //     }
//     //     let mut tags = IRCTags::default();

//     //     if let Some(matches) = privmsg_filter.captures(&value) {
//     //         if let Some(tags_match) = matches.get(1) {
//     //             tags.add_tag(tags_match.as_str());
//     //         }
//     //         return IRCMessage {
//     //             tags: tags,
//     //             command: matches.get(3).unwrap().as_str().try_into().unwrap_or(IRCCommand::UnsupportedError),
//     //             channel: Some(matches.get(4).unwrap().as_str().to_owned()),
//     //             nick: Some(matches.get(2).unwrap().as_str().to_owned()),
//     //             message: Some(matches.get(5).unwrap().as_str().to_owned()),
//     //         };
//     //     } else if let Some(matches) = twitchmsg_filter.captures(&value) {
//     //         // matches.get(1).unwrap().as_str().split(';').for_each(|s| tags.add_tag(s).expect(&value));
//     //         return IRCMessage {
//     //             tags: tags,
//     //             command: matches.get(2).unwrap().as_str().try_into().unwrap_or(IRCCommand::UnsupportedError),
//     //             channel: Some(matches.get(3).unwrap().as_str().to_owned()),
//     //             nick: None,
//     //             message: matches.get(4).and_then(|s| Some(s.as_str().to_string())),
//     //         }
//     //     } else if let Some(matches) = miscmsg_filter.captures(&value) {
//     //         return IRCMessage {
//     //             tags: tags,
//     //             command: matches.get(1).unwrap().as_str().try_into().unwrap_or(IRCCommand::UnsupportedError),
//     //             channel: None,
//     //             nick: None,
//     //             message: None,
//     //         };
//     //     } else {
//     //         println!("failed to parse message:\n{}", value);
//     //         panic!();
//     //         return IRCMessage::default();
//     //     }
//     // }
// }

impl TryFrom<&str> for IRCMessage {
    type Error = IRCMessageParseError;

    fn try_from(msg: &str) -> Result<IRCMessage, IRCMessageParseError> {
        use IRCMessageParseError::*;

        let chars: Vec<char> = msg.chars().collect();
        let mut tags = IRCTags::default();

        let mut tags_str = String::new();
        let source_str: String;
        let command_str: String;
        let mut message_str = String::new();

        let mut command: Option<IRCCommand> = None;
        let mut channel: Option<String> = None;
        let mut idx = 0;

        if chars[idx] == '@' {
            if let Some(end_of_tags) = chars.iter().position(|c| *c == ' ') {
                tags_str = chars[1..end_of_tags].iter().collect();
                idx = end_of_tags + 1;
            } else {
                return Err(TagParseError);
            }
        }

        if chars[idx] == ':' {
            if let Some(end_of_source) = chars[idx..].iter().position(|c| *c == ' ') {
                source_str = chars[idx..][1..end_of_source].iter().collect();
                idx = end_of_source + idx;
            } else {
                return Err(StructureError);
            }
        } else if chars[idx..].starts_with(&['P', 'I', 'N', 'G']) {
            return Ok(IRCMessage {
                tags,
                nick: None,
                command: IRCCommand::Ping,
                channel: None,
                message: None,
            })
        } else {
            return Err(NoSource);
        }

        if let Some(start_of_message) = chars[idx..].iter().position(|c| *c == ':') {
            command_str = chars[idx..][1..start_of_message-1].iter().collect();
            message_str = chars[idx..][start_of_message+1..].iter().collect();
            message_str = String::from(message_str.trim());
        } else {
            command_str = chars[idx..].iter().collect();
        }

        tags.add_from_string(&tags_str);

        for comm in command_str.split(' ') {
            if comm.starts_with('#') {
                channel = Some(comm[1..].to_string());
            } else if let Ok(parsed_command) = IRCCommand::try_from(comm) {
                command = Some(parsed_command);
            }
        }
        if command.is_none() {
            return Err(NoCommand);
        }

        return Ok(IRCMessage {
            tags: tags,
            nick: nick_from_source(&source_str),
            command: command.unwrap(),
            channel: channel,
            message: {
                if message_str.is_empty() {
                    None
                } else {
                    Some(message_str)
                }
            }
        });

        fn nick_from_source(source: &str) -> Option<String> {
            if let Some((nick, _)) = source.split_once('!') {
                return Some(nick.to_string());
            } else {
                return None;
            }
        }
    }
}

impl Default for IRCMessage {
    fn default() -> Self {
        Self {
            tags: IRCTags::default(),
            command: IRCCommand::Useless,
            channel: None,
            nick: None,
            message: None,
        }
    }
}

impl Display for IRCMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.channel.is_some() && self.nick.is_some() && self.message.is_some() {
            write!(
                f,
                "#{} {}: {}",
                self.channel.as_ref().unwrap(),
                self.nick.as_ref().unwrap(),
                self.message.as_ref().unwrap()
            )
        } else {
            write!(f, "{}", format!("{:#?}", self))
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(into = "String"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IRCCommand {
    Join,
    Part,
    Notice,
    ClearChat,
    ClearMsg,
    HostTarget,
    PrivMsg,
    Whisper,
    Ping,
    Cap,
    GlobalUserState,
    UserState,
    RoomState,
    UserNotice,
    Reconnect,
    UnsupportedError,
    AuthSuccessfull,
    UserList,
    Useless,
}

#[derive(Debug)]
pub enum IRCCommandError {
    Failed,
}

impl TryFrom<&str> for IRCCommand {
    type Error = IRCCommandError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        match input {
            "JOIN" => Ok(Self::Join),
            "PART" => Ok(Self::Part),
            "NOTICE" => Ok(Self::Notice),
            "CLEARCHAT" => Ok(Self::ClearChat),
            "CLEARMSG" => Ok(Self::ClearMsg),
            "HOSTTARGET" => Ok(Self::HostTarget),
            "PRIVMSG" => Ok(Self::PrivMsg),
            "PING" => Ok(Self::Ping),
            "CAP" => Ok(Self::Cap),
            "GLOBALUSERSTATE" => Ok(Self::GlobalUserState),
            "USERSTATE" => Ok(Self::UserState),
            "ROOMSTATE" => Ok(Self::RoomState),
            "USERNOTICE" => Ok(Self::UserNotice),
            "RECONNECT" => Ok(Self::Reconnect),
            "WHISPER" => Ok(Self::Whisper),
            "421" => Ok(Self::UnsupportedError),
            "353" => Ok(Self::UserList),
            "366" => Ok(Self::UserList),
            "001" => Ok(Self::AuthSuccessfull),
            "002" => Ok(Self::Useless),
            "003" => Ok(Self::Useless),
            "004" => Ok(Self::Useless),
            "375" => Ok(Self::Useless),
            "372" => Ok(Self::Useless),
            "376" => Ok(Self::Useless),
             _ => Err(IRCCommandError::Failed),
            }
    }
}

impl From<IRCCommand> for String {
    fn from(value: IRCCommand) -> Self {
        return String::from(match value {
            IRCCommand::Join => "JOIN",
            IRCCommand::Part => "PART",
            IRCCommand::Notice => "NOTICE",
            IRCCommand::ClearChat => "CLEARCHAT",
            IRCCommand::ClearMsg => "CLEARMSG",
            IRCCommand::HostTarget => "HOSTTARGET",
            IRCCommand::PrivMsg => "PRIVMSG",
            IRCCommand::Whisper => "WHISPER",
            IRCCommand::Ping => "PING",
            IRCCommand::Cap => "CAP",
            IRCCommand::GlobalUserState => "GLOBALUSERSTATE",
            IRCCommand::UserState => "USERSTATE",
            IRCCommand::RoomState => "ROOMSTATE",
            IRCCommand::UserNotice => "USERNOTICE",
            IRCCommand::Reconnect => "RECONNECT",
            IRCCommand::UnsupportedError => "421",
            IRCCommand::AuthSuccessfull => "001",
            IRCCommand::UserList => "353",
            IRCCommand::Useless => "",
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct IRCTags {
    pub tags: HashMap<String, String>
}

#[derive(Debug)]
pub enum IRCTagParseError {
    TagStructureParseError(String),
    IdentifierNotRecognized(String),
    ContentParseFailed(String),
}

impl IRCTags {
    // fn add_tag(&mut self, value: &str) -> Result<(), IRCTagParseError> {
    //     let parse_bool = |val: &str| -> bool {
    //         val.parse::<i32>().unwrap() > 0
    //     };

    //     let char_to_int = |input :char| -> u8 {
    //         match input {
    //             '0' => 0x0,
    //             '1' => 0x1,
    //             '2' => 0x2,
    //             '3' => 0x3,
    //             '4' => 0x4,
    //             '5' => 0x5,
    //             '6' => 0x6,
    //             '7' => 0x7,
    //             '8' => 0x8,
    //             '9' => 0x9,
    //             'A' => 0xA,
    //             'B' => 0xB,
    //             'C' => 0xC,
    //             'D' => 0xD,
    //             'E' => 0xE,
    //             'F' => 0xF,
    //             _ => panic!()
    //         }
    //     };

    //     let (identifier, content) = match value.split_once("=") {
    //         Some(x) => (x.0, x.1),
    //         None => return Err(
    //             IRCTagParseError::TagStructureParseError(value.to_owned())
    //         ),
    //     };
    //     match identifier {
    //         "badge-info" => Ok(self.badge_info = Some(content.to_string())),
    //         "badges" => {
    //             if content.is_empty() {
    //                 return Ok(self.badges = None)
    //             }
    //             let splits = content.split(",");
    //             let mut badges = Vec::new();
    //             for split in splits {
    //                 if let Some((name, version)) = split.split_once('/') {
    //                     badges.push(Badge {
    //                         name: name.to_string(),
    //                         version: version.parse().unwrap_or(0)
    //                     });
    //                 } else {
    //                     return Err(
    //                         IRCTagParseError::ContentParseFailed(
    //                             content.to_string()
    //                         )
    //                     );
    //                 }
    //             }
    //             Ok(self.badges = Some(badges))
    //         },
    //         "color" => {
    //             let individuals: Vec<char> = content.chars().collect();
    //             if content.is_empty() {
    //                 return Ok(self.color = None)
    //             }
    //             Ok(self.color = Some([
    //                 char_to_int(individuals[1]) * 16 + char_to_int(individuals[2]),
    //                 char_to_int(individuals[3]) * 16 + char_to_int(individuals[4]),
    //                 char_to_int(individuals[5]) * 16 + char_to_int(individuals[6]),
    //             ]))
    //         },
    //         "display-name" => Ok(self.display_name = Some(content.to_string())),
    //         "emotes" => Ok(self.emotes = Some(vec![content.to_string()])),
    //         "emote-only" => Ok(self.emote_only = Some(true)),
    //         "followers-only" => Ok(self.followers_only = Some(content.parse().unwrap())),
    //         "r9k" => Ok(self.r9k = Some(parse_bool(content))),
    //         "slow" => Ok(self.slow = Some(content.parse().unwrap())),
    //         "subs-only" => Ok(self.subs_only = Some(parse_bool(content))),
    //         "ban-duration" => Ok(self.ban_duration = Some(content.parse().unwrap())),
    //         "target-user-id" => Ok(self.target_user_id = Some(content.to_string())),
    //         "target-msg-id" => Ok(self.target_msg_id = Some(content.to_string())),
    //         "msg-id" => Ok(self.msg_id = Some(content.to_string())),
    //         "first-msg" => Ok(self.first_msg = Some(parse_bool(content))),
    //         "room-id" => Ok(self.room_id = Some(content.to_string())),
    //         "flags" => Ok(self.flags = Some(content.to_string())),
    //         "client-nonce" => Ok(self.client_nonce = Some(content.to_string())),
    //         "id" => Ok(self.msg_id = Some(content.to_string())),
    //         "returning-chatter" => Ok(self.returning_chatter = Some(parse_bool(content))),
    //         "vip" => Ok(self.is_vip = Some(parse_bool(content))),
    //         "mod" => Ok(self.is_mod = Some(parse_bool(content))),
    //         "turbo" => Ok(self.is_turbo = Some(parse_bool(content))),
    //         "subscriber" => Ok(self.subscriber = Some(parse_bool(content))),
    //         "tmi-sent-ts" => Ok(self.timestamp = Some(content.parse().unwrap())),
    //         "user-id" => Ok(self.user_id = Some(content.to_string())),
    //         "login" => Ok(self.login = Some(content.to_string())),
    //         "bits" => Ok(self.bits = Some(content.parse().unwrap())),
    //         "user-type" => Ok(self.user_type = Some(content.to_string())),
    //         "system-msg" => Ok(self.system_msg = Some(content.to_string().replace("\\s", " "))),
    //         "reply-parent-msg-id" => Ok(self.reply_parent_msg_id = Some(content.to_string())),
    //         "reply-parent-user-id" => Ok(self.reply_parent_user_id = Some(content.to_string())),
    //         "reply-parent-user-login" => Ok(self.reply_parent_user_login = Some(content.to_string())),
    //         "reply-parent-display-name" => Ok(self.reply_parent_display_name = Some(content.to_string())),
    //         "reply-parent-msg-body" => Ok(self.reply_parent_msg_body = Some(content.to_string())),
    //         "msg-param-cumulative-months" => Ok(self.sub_cumulative_months = Some(content.parse().unwrap())),
    //         "msg-param-displayName" => Ok(self.raid_display_name = Some(content.to_string())),
    //         "msg-param-login" => Ok(self.raid_login = Some(content.to_string())),
    //         "msg-param-months" => Ok(self.sub_months_gifted = Some(content.parse().unwrap())),
    //         "msg-param-promo-gift-total" => Ok(self.sub_promo_total_gifted = Some(content.parse().unwrap())),
    //         "msg-param-promo-name" => Ok(self.sub_promo_name = Some(content.to_string())),
    //         "msg-param-recipient-display-name" => Ok(self.sub_recipient_display_name = Some(content.to_string())),
    //         "msg-param-recipient-id" => Ok(self.sub_recipient_id = Some(content.to_string())),
    //         "msg-param-recipient-user-name" => Ok(self.sub_recipient_user_name = Some(content.to_string())),
    //         "msg-param-sender-login" => Ok(self.sub_sender_login = Some(content.to_string())),
    //         "msg-param-sender-name" => Ok(self.sub_sender_display_name = Some(content.to_string())),
    //         "msg-param-should-share-streak" => Ok(self.sub_should_share_streak = Some(parse_bool(content))),
    //         "msg-param-streak-months" => Ok(self.sub_month_streak = Some(content.parse().unwrap())),
    //         "msg-param-sub-plan" => Ok(self.sub_tier = Some(SubTier::from(content.to_string()))),
    //         "msg-param-sub-plan-name" => Ok(self.sub_plan_name = Some(content.to_string().replace("\\s", " "))),
    //         "msg-param-viewerCount" => Ok(self.raid_viewer_count = Some(content.parse().unwrap())),
    //         "msg-param-ritual-name" => Ok(self.ritual_name = Some(content.to_string())),
    //         "msg-param-threshold" => Ok(self.bit_badge_tier = Some(content.to_string())),
    //         "msg-param-gift-months" => Ok(self.sub_single_months_gifted = Some(content.parse().unwrap())),
    //         "msg-param-multimonth-duration" => Ok(self.sub_multi_month_duration = Some(content.parse().unwrap())),
    //         "msg-param-multimonth-tenure" => Ok(self.sub_multi_month_tenure = Some(content.parse().unwrap())),
    //         "msg-param-was-gifted" => Ok(self.sub_was_gifted = Some(content.parse().unwrap())),
    //         "msg-param-mass-gift-count" => Ok(self.sub_mass_gift_count = Some(content.parse().unwrap())),
    //         "msg-param-sender-count" => Ok(self.sub_overall_months_gifted = Some(content.parse().unwrap())),
    //         "msg-param-origin-id" => Ok(self.sub_origin_id = Some(content.to_string())),
    //         "msg-param-profileImageURL" => Ok(self.raid_profile_image_url = Some(content.to_string())),
    //         "msg-param-fun-string" => Ok(self.sub_fun_string = Some(content.to_string())),
    //         "msg-param-prior-gifter-anonymous" => Ok(self.sub_pay_forward_prior_gifter_anonymous = Some(content.parse().unwrap())),
    //         "msg-param-prior-gifter-display-name" => Ok(self.sub_pay_forward_prior_gifter_display_name = Some(content.to_string())),
    //         "msg-param-prior-gifter-id" => Ok(self.sub_pay_forward_prior_gifter_id = Some(content.to_string())),
    //         "msg-param-prior-gifter-user-name" => Ok(self.sub_pay_forward_prior_gifter_user_name = Some(content.to_string())),
    //         "msg-param-color" => Ok(self.announcement_color = Some(content.to_string())),
    //         x => Err(IRCTagParseError::IdentifierNotRecognized(x.to_owned())),
    //     }
    // }

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
        self.tags.reserve(tags.clone().count());
        for tag in tags {
            let (id, value) = tag.split_once('=').unwrap();
            self.tags.insert(id.to_string(), value.to_string());
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
            match input {
                '0' => 0x0,
                '1' => 0x1,
                '2' => 0x2,
                '3' => 0x3,
                '4' => 0x4,
                '5' => 0x5,
                '6' => 0x6,
                '7' => 0x7,
                '8' => 0x8,
                '9' => 0x9,
                'A' => 0xA,
                'B' => 0xB,
                'C' => 0xC,
                'D' => 0xD,
                'E' => 0xE,
                'F' => 0xF,
                _ => panic!()
            }
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

    pub fn get_timestamp(&self) -> Option<i64> {
        self.get_value("tmi-sent-ts")?.parse::<i64>().ok()
    }

    pub fn get_message_id(&self) -> Option<&str> {
        self.get_value("id")
    }

    pub fn get_sender_id(&self) -> Option<&str> {
        self.get_value("user-id")
    }
}

impl From<IRCTags> for String {
    fn from(tags: IRCTags) -> Self {
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
