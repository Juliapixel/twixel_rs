use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use twixel_core::irc_message::{error::RawIrcMessageParseError, command::IrcCommand, raw::RawIrcMessage, owned::OwnedIrcMessage, tags::OwnedTag, prefix::OwnedPrefix};

#[inline]
fn deserialize_irc_message(msg: &str) -> Result<RawIrcMessage, RawIrcMessageParseError> {
    RawIrcMessage::try_from(msg)
}

#[cfg(test)]
const SHIT_TON: &'static str = include_str!("../../logs/logs.txt");

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

criterion_group!(benches, deserialize_shit_ton, build_and_format_owned_messages);
criterion_main!(benches);
