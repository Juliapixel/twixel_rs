use std::{path::PathBuf, sync::LazyLock};

use clap::Parser;

pub static ARGS: LazyLock<Args> = LazyLock::new(|| {
    let dotenv_found = dotenvy::dotenv().is_ok();
    if !dotenv_found {
        log::warn!(".env file was not found")
    }

    Args::parse()
});

#[derive(clap::Parser)]
pub struct Args {
    #[arg(required = true)]
    pub channels: Vec<String>,
    #[arg(long, env = "TWIXEL_CONFIG")]
    #[cfg_attr(debug_assertions, arg(default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")))]
    pub config: PathBuf,
}
