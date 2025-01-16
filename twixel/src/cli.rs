use std::sync::LazyLock;

use clap::Parser;

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(clap::Parser)]
pub struct Args {
    #[arg(required = true)]
    pub channels: Vec<String>,
}
