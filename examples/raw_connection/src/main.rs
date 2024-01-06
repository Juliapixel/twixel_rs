use std::sync::{Arc, Mutex};

use env_logger::Env;
use twitch_irc::{connection::{Connection, ConnectionError}, user::ClientInfo, auth::Auth, irc_message::{owned::OwnedIrcMessage, command::IrcCommand}};

#[tokio::main]
async fn main() -> Result<(), ConnectionError> {
    env_logger::init_from_env(
        Env::new().default_filter_or("DEBUG")
    );

    let client_info = ClientInfo::new(Auth::Anonymous);
    let mut conn = Connection::new(Arc::new(Mutex::new(client_info)));

    conn.start().await?;
    conn.send(OwnedIrcMessage {
        tags: None,
        prefix: None,
        command: IrcCommand::Join,
        params: vec!["#julialuxel,#xqc,#pokelawls,#forsen,#erobb221,#psp1g,#dizzy,#hasanabi,#esfandtv,#omie,#summit1g,#shroud".into()]
    }).await?;
    loop {
        let recv = conn.receive().await?;
        for i in recv {
            println!("{}", serde_json::to_string_pretty(&i).unwrap());
        }
    }

    Ok(())
}
