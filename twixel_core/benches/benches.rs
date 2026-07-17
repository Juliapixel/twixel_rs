use std::hint::black_box;

use divan::{Bencher, counter::BytesCount};
use mimalloc::MiMalloc;
use twixel_core::irc_message::{
    builder::MessageBuilder, message::IrcMessage, prefix::OwnedPrefix, tags::OwnedTag,
};

#[cfg(test)]
const SHIT_TON: &str = include_str!("../../logs/logs.txt");
const COUNT: usize = 20000;

// #[global_allocator]
// static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc{});

#[global_allocator]
static ALLOC: MiMalloc = MiMalloc {};

#[divan::bench(threads = [0, 1], min_time = 1)]
fn deserialize_shit_ton(bencher: Bencher) {
    bencher
        .with_inputs(|| SHIT_TON.lines().take(COUNT).collect::<Vec<&str>>())
        .input_counter(|i| i.len())
        .input_counter(|i| BytesCount::new(i.iter().fold(0, |r, i| r + i.len())))
        .bench_local_values(move |messages| {
            for i in messages.into_iter() {
                IrcMessage::try_from(black_box(i)).unwrap();
            }
        });
}

#[divan::bench]
fn build_and_format_owned_messages() {
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
}

fn main() {
    divan::main();
}
