use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use twitch_irc::irc_message::{error::RawIrcMessageParseError, command::IrcCommand, raw::RawIrcMessage, owned::OwnedIrcMessage, tags::OwnedTag, prefix::OwnedPrefix};

#[inline]
fn deserialize_irc_message(msg: &str) -> Result<RawIrcMessage, RawIrcMessageParseError> {
    RawIrcMessage::try_from(msg)
}

#[cfg(test)]
const SHIT_TON: &'static str = include_str!("../logs/logs.txt");

fn deserialize_shit_ton(c: &mut Criterion) {
    let messages: Vec<&str> = SHIT_TON.lines().collect();

    c.bench_function("Parse a Bunch of xQc's Chat's Logs", |b| b.iter_custom(|iterations| {
        let start = std::time::Instant::now();
        for i in 0..iterations {
            black_box(deserialize_irc_message(&messages[i as usize % messages.len()])).unwrap();
        }
        return start.elapsed();
    }));
}

fn format_irc_message(c: &mut Criterion) {
    static TEST_MSG: &str = "@badge-info=subscriber/37;badges=subscriber/36,moments/3;client-nonce=495183297aef9a105dc62a29b36fbc99;color=#FF4500;display-name=luistacoz;emotes=;first-msg=0;flags=;id=db6c8616-7716-4150-9109-59172ec9e5d9;mod=0;returning-chatter=0;room-id=71092938;subscriber=1;tmi-sent-ts=1680318925519;turbo=0;user-id=84483850;user-type= :luistacoz!luistacoz@luistacoz.tmi.twitch.tv PRIVMSG #xqc :HYPERDANSGAME LOOKS VILE\r\n";
    let parsed = RawIrcMessage::try_from(black_box(TEST_MSG)).unwrap();
    c.bench_function("Format IRC Message", |b| {
        b.iter(|| {
            black_box(parsed.to_string());
        });
    });
}

fn build_and_format_owned_messages(c: &mut Criterion) {
    c.bench_function("Build and format OwnedIrcMessage", |b| {
        b.iter(|| {
            let owned = black_box(OwnedIrcMessage {
                tags: Some(vec![
                    (OwnedTag::Unknown("tag1".into()), "val1".into()),
                    (OwnedTag::Unknown("tag2".into()), "val2".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                    (OwnedTag::Unknown("tag3".into()), "val3".into()),
                ]),
                prefix: Some(
                    OwnedPrefix::Full {
                        nickname: "juliapixel".into(),
                        username: "julia".into(),
                        host: "juliapixel.com".into()
                    }
                ),
                command: IrcCommand::PrivMsg,
                params: vec![
                    "#juliapixel".into(),
                    ":hi hello there!".into()
                ],
            });
            black_box(owned.to_string());
        })
    });
}

criterion_group!(benches, deserialize_shit_ton, format_irc_message, build_and_format_owned_messages);
criterion_main!(benches);
