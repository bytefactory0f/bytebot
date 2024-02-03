mod config;
mod commands;

use std::error::Error;
use std::collections::HashMap;
use config::Config;
use commands::CommandConfig;
use twitch_client_rs::twitch_client::{TwitchClient, Capability};
use twitch_client_rs::irc::IRCMessage;
use twitch_client_rs::credentials::Credentials;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config_file = std::fs::File::open("bytebot.yaml")?;
    let config: Config = serde_yaml::from_reader(config_file)?;

    let command_file = std::fs::File::open("commands.yaml")?;
    let command_config: CommandConfig = serde_yaml::from_reader(command_file)?;
    let commands = HashMap::from(command_config);

    let channel = "bytefactory";

    let credentials = Credentials {
        refresh_token: config.refresh_token,
        client_id: config.client_id,
        client_secret: config.client_secret,
    };

    let mut twitch_client = TwitchClient::new(credentials, config.nick)?;

    // Get/refresh the access token
    twitch_client.update_access_token().await?;

    // Open a websocket connection to twitch
    twitch_client.connect().await?;

    // Authenticate with the twitch IRC server
    twitch_client.authenticate().await?;

    twitch_client.cap_req(&[Capability::Tags]).await?;

    // Joint a twitch chat
    twitch_client.join(channel).await?;

    while let Some(irc_message) = twitch_client.next().await {
        dbg!(&irc_message);
        match irc_message? {
            IRCMessage::Ping(msg) => {
                println!("GOT PING: {msg}");
                twitch_client.pong(msg.as_str()).await?;
            },
            IRCMessage::Privmsg { message, is_mod, .. } => {
                if let Some((command_str, args)) = get_message_components(&message) {
                    if let Some(command) = commands.get(command_str) {
                        if let Some(reply) = command.get_reply(&args, is_mod) {
                            twitch_client.privmsg(channel, &reply).await?
                        }
                    }
                }
            }
            _ => {}
        }
    }

    println!("DONE");

    Ok(())
}

fn get_message_components(message: &str) -> Option<(&str, Vec<&str>)> {
    let parts: Vec<&str> = message.split_whitespace().collect();
    let command_str = parts.first()?;
    let first = *command_str;
    let args = parts[1..].to_vec();
    Some((first, args))
}
