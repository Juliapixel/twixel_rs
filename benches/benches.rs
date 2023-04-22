use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use twitch_irc::irc_message::{IRCMessage, IRCMessageParseError, IRCTags, IRCMessageFormatter, IRCCommand};

#[inline]
fn deserialize_irc_message(msg: &str) -> Result<IRCMessage, IRCMessageParseError> {
    TryInto::<IRCMessage>::try_into(msg)
}

fn deserialize_shit_ton(c: &mut Criterion) {
    let files = std::fs::read_dir("F:\\Julia\\twitch_irc\\logs").unwrap();
    let mut messages: Vec<String> = Vec::new();

    for file in files {
        let logs = std::fs::read_to_string(file.unwrap().path()).unwrap();
        for line in logs.lines() {
            messages.push(line.to_string());
        }
    }

    c.bench_function("Deserialize a Bunch of xQc's Chat's Logs", |b| b.iter_custom(|iterations| {
        let start = std::time::Instant::now();
        for i in 0..iterations {
            deserialize_irc_message(&messages[i as usize % messages.len()]).unwrap();
        }
        return start.elapsed();
    }));
}

fn format_irc_message(c: &mut Criterion) {
    c.bench_function("Format IRC Message", |b| {
        b.iter(|| {
            let a = IRCMessage {
                tags: IRCTags::new_with_tags(&[
                    ("tag1", "key1"),
                    ("tag2", "key2"),
                    ("tag3", "key3"),
                    ("tag4", "key4"),
                    ("tag5", "key5"),
                    ("tag6", "key6"),
                    ("tag7", "key7"),
                    ("tag8", "key8"),
                    ("tag9", "key9"),
                    ("tag10", "key10"),
                    ("tag11", "key11"),
                    ("tag12", "key12"),
                ]),
                nick: None,
                command: IRCCommand::PrivMsg,
                channel: Some(String::from("julialuxel")),
                message: Some(String::from("julialuxel")),
            }.to_string(IRCMessageFormatter::Client);
            black_box(a);
        });
    });
}

fn parse_tags(c: &mut Criterion) {
    c.bench_function("Add Tags", |b| {
        b.iter(|| {
            let mut tags = IRCTags::new();
            tags.add_from_string("@badge-info=;badges=moments/2;client-nonce=7f1a51ec7a1a6a628a26728994fb4f93;color=#FFFFFF;display-name=3dge;emotes=;first-msg=0;flags=;id=e447f8cc-35bc-4dd7-9e80-06cad3cd9e67;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1680318865347;turbo=0;user-id=104665403;user-type=");
        });
    });
}

criterion_group!(benches, deserialize_shit_ton, parse_tags, format_irc_message);
criterion_main!(benches);
