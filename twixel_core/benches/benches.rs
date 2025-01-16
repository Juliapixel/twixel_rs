use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use twixel_core::irc_message::{
    builder::MessageBuilder, message::IrcMessage, prefix::OwnedPrefix, tags::OwnedTag,
};

#[cfg(test)]
const SHIT_TON: &str = include_str!("../../logs/logs.txt");

fn deserialize_shit_ton(c: &mut Criterion) {
    let messages: Vec<&str> = SHIT_TON.lines().take(1000).collect();

    c.bench_function("Parse a Bunch of xQc's Chat's Logs", |b| {
        b.iter_custom(|iterations| {
            let start = std::time::Instant::now();
            for _ in 0..iterations {
                for message in messages.iter() {
                    IrcMessage::new(black_box(*message).into()).unwrap();
                }
            }
            start.elapsed()
        })
    });
}

fn build_and_format_owned_messages(c: &mut Criterion) {
    c.bench_function("Build and format MessageBuilder", |b| {
        b.iter(|| {
            let owned = black_box(
                MessageBuilder::privmsg("juliapixel", "hi im julia!")
                    .add_tag(OwnedTag::Color, "#ffffff")
                    .add_tag(OwnedTag::DisplayName, "Juliapixel")
                    .add_tag(OwnedTag::Id, "12345678")
                    .prefix(OwnedPrefix::OnlyHostname {
                        host: "juliapixel.com".into(),
                    }),
            );
            black_box(owned.build());
        })
    });
}

criterion_group!(
    benches,
    deserialize_shit_ton,
    build_and_format_owned_messages
);
criterion_main!(benches);
