use super::{prefix::Prefix, tags::IrcTags, command::IrcCommand, error::RawIrcMessageParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIrcMessage {
    pub(crate) tags: Option<IrcTags>,
    pub(crate) prefix: Option<Prefix>,
    pub(crate) command: IrcCommand,
    pub(crate) params: Vec<String>,
}

impl TryFrom<&str> for RawIrcMessage {
    type Error = RawIrcMessageParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use RawIrcMessageParseError as E;

        let mut state = value;

        // parses tags if there are any, and sets the start of `state` to right
        // right after the trailing space after the tags
        let tags = if state.starts_with('@') {
            let tag_end = state.find(' ');
            if tag_end.is_none() { return Err(E::StructureError); }
            let mut tmp = IrcTags::new();
            tmp.add_from_string(&state[0..tag_end.unwrap()]);
            state = &state[tag_end.unwrap()+1..];
            Some(tmp)
        } else {
            None
        };

        // parses the prefix, if there is one and then sets the start of `state`
        // to the first character of the command
        let prefix = if state.starts_with(':') {
            if let Some(prefix_end) = state.find(' ') {
                let out = Prefix::from(&state[1..prefix_end]);
                state = &state[prefix_end+1..];
                Some(out)
            } else {
                return Err(E::StructureError);
            }
        } else {
            None
        };

        // splits the command from its parameters (if present)
        let (cmd, params_str) = match state.split_once(' ') {
            Some(s) => (s.0, Some(s.1)),
            None => (state, None),
        };

        let command = IrcCommand::try_from(cmd).unwrap();

        let mut params = Vec::new();

        if let Some(params_some) = params_str {
            let mut params_some = params_some;
            loop {
                if let Some(pos) = params_some.find(' ') {
                    params.push(String::from(&params_some[0..pos]));
                    if let Some(second_part) = params_some.get(pos+1..) {
                        if second_part.starts_with(':') {
                            params.push(String::from(second_part));
                            break;
                        } else {
                            params_some = second_part;
                        }
                    } else {
                        break;
                    }
                } else {
                    params.push(String::from(params_some.trim()));
                    break;
                }
            }

        }

        return Ok(Self {
            tags: tags,
            prefix: prefix,
            command: command,
            params: params
        })
    }
}

impl From<RawIrcMessage> for String {
    fn from(value: RawIrcMessage) -> Self {
        todo!();
        // let mut out = format!("{} :{} {} {}", value.tags, value.prefix, value.command, value.params);
    }
}
