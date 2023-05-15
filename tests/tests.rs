use twitch_irc::irc_message::{IRCMessage, IRCCommand, IRCTags, IRCMessageFormatter};

const DIVERSE_MESSAGES: &str =
r":tmi.twitch.tv 001 placeholdername :Welcome, GLHF!
:tmi.twitch.tv 002 placeholdername :Your host is tmi.twitch.tv
:tmi.twitch.tv 003 placeholdername :This server is rather new
:tmi.twitch.tv 004 placeholdername :-
:tmi.twitch.tv 375 placeholdername :-
:tmi.twitch.tv 372 placeholdername :You are in a maze of twisty passages, all alike.
:tmi.twitch.tv 376 placeholdername :>
:tmi.twitch.tv CAP * ACK :twitch.tv/commands twitch.tv/tags
:placeholdername!placeholdername@placeholdername.tmi.twitch.tv JOIN #test
@emote-only=0;followers-only=10;r9k=0;room-id=91067577;slow=0;subs-only=0 :tmi.twitch.tv ROOMSTATE #test
:placeholdername.tmi.twitch.tv 353 placeholdername = #test :placeholdername
:placeholdername.tmi.twitch.tv 366 placeholdername #test :End of /NAMES list
@badge-info=;badges=moments/2;client-nonce=7f1a51ec7a1a6a628a26728994fb4f93;color=#FFFFFF;display-name=3dge;emotes=;first-msg=0;flags=;id=e447f8cc-35bc-4dd7-9e80-06cad3cd9e67;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1680318865347;turbo=0;user-id=104665403;user-type= :3dge!3dge@3dge.tmi.twitch.tv PRIVMSG #xqc :*conquers and colonizes 25% of the world and doesn't use seasoning or spices* British  󠀀
@ban-duration=15;room-id=71092938;target-user-id=466194973;tmi-sent-ts=1680318869646 :tmi.twitch.tv CLEARCHAT #xqc :iankers
PING :tmi.twitch.tv
";

#[test]
fn irc_message_deserialization() {
    let lines: Vec<&str> = DIVERSE_MESSAGES.lines().collect();
    let mut messages: Vec<IRCMessage> = Vec::new();
    for line in lines {
        let msg = TryInto::<IRCMessage>::try_into(line).unwrap();
        messages.push(msg);
    }
    assert_eq!(
        messages[0],
        IRCMessage {
            tags: IRCTags::default(),
            nick: None,
            command: IRCCommand::AuthSuccessfull,
            channel: None,
            message: Some(String::from("Welcome, GLHF!"))
        }
    );
    // TODO: add more assertions to make sure the deserialization is correct
}

#[cfg(test)]
pub const SHIT_TON: &'static str = include_str!("../logs/logs.txt");

#[test]
fn test_a_shit_ton() {
    let messages: Vec<&str> = SHIT_TON.lines().collect();

    for msg in messages {
        TryInto::<IRCMessage>::try_into(msg).expect(&msg);
    }
}

#[test]
fn irc_message_formatting() {
    let priv_msg = IRCMessage {
        tags: IRCTags::new(),
        nick: None,
        command: IRCCommand::PrivMsg,
        channel: Some(String::from("julialuxel")),
        message: Some(String::from("message 123")),
    }.to_string(IRCMessageFormatter::Client);

    let server_priv_msg = IRCMessage {
        tags: IRCTags::new(),
        nick: Some(String::from("julialuxel")),
        command: IRCCommand::PrivMsg,
        channel: Some(String::from("julialuxel")),
        message: Some(String::from("message 123")),
    }.to_string(IRCMessageFormatter::Server);


    assert_eq!(priv_msg, "PRIVMSG #julialuxel :message 123");
    assert_eq!(server_priv_msg, ":julialuxel!julialuxel@julialuxel.tmi.twitch.tv PRIVMSG #julialuxel :message 123");
}
