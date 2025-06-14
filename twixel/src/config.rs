use std::{path::PathBuf, sync::LazyLock};

use serde::Deserialize;

use crate::cli::ARGS;

pub static CONFIG: LazyLock<Config> = LazyLock::new(get_config);

#[derive(Debug, Deserialize)]
pub struct Config {
    pub twitch: Twitch,
    pub database: Database,
    pub openai: OpenAi,
}

#[derive(Debug, Deserialize)]
pub struct Twitch {
    pub token: String,
    pub login: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct OpenAi {
    pub api_key: Option<String>,
}

fn get_config() -> Config {
    let config_path: PathBuf = ARGS.config.clone();

    let config = match config::Config::builder()
        .add_source(config::File::from(config_path))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            panic!("{e}")
        }
    };

    match config.try_deserialize() {
        Ok(c) => c,
        Err(e) => panic!("{e}"),
    }
}
