use env_logger::Env;
use futures_util::TryStreamExt;
use twixel_core::{
    connection::{error::PoolError, pool::ConnectionPool},
    auth::Auth,
    irc_message::{command::IrcCommand, tags::OwnedTag}
};

#[tokio::main]
async fn main() -> Result<(), PoolError> {
    env_logger::init_from_env(
        Env::new().default_filter_or("DEBUG")
    );

    const CHANS: &[&str] = &[
        "julialuxel", "xqc", "pokelawls", "forsen", "erobb221", "psp1g",
        "dizzy", "hasanabi", "esfandtv", "omie", "summit1g", "shroud",
        "emiru", "zoil"
    ];

    let mut conn = ConnectionPool::new(CHANS.iter().copied(), Auth::Anonymous).await.unwrap();

    loop {
        let recv = conn.try_next().await?;
        for i in recv.map(|r| r.0).iter().flatten() {
            if i.get_command() == IrcCommand::PrivMsg {
                println!(
                    "{} {}: {}",
                    i.get_param(0).unwrap(),
                    i.get_tag(OwnedTag::DisplayName).unwrap(),
                    i.get_param(1).unwrap().split_at(1).1
                )
            }
        }
    }
}
