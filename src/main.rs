mod commands;
mod config;

use commands::CommandConfig;
use config::Config;
use std::collections::HashMap;
use std::error::Error;
use twitch_client_rs::credentials::Credentials;
use twitch_client_rs::irc::IRCMessage;
use twitch_client_rs::twitch_client::{Capability, TwitchClient};

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

    let mut twitch_client = TwitchClient::new(credentials, config.nick, true);

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

        match irc_message {
            Ok(message) => {
                if let IRCMessage::Privmsg { message, user_context, .. } = message {
                    if let Some((command_str, args)) = get_message_components(&message) {
                        if let Some(command) = commands.get(command_str) {
                            if let Some(reply) = command.get_reply(&args, user_context) {
                                twitch_client.privmsg(channel, &reply).await?
                            }
                        }
                    }
                }
            }
            Err(err) => {
                println!("error: {}", err)
            }
        }
    }

    println!("DONE");

    Ok(())
}

/// Splits a message into components, with the first component being
/// treated as a potential command, and the rest as arguments to that
/// command.
/// Ex: !shoutout hello  world!
///     command   arg[0] arg[1]
fn get_message_components(message: &str) -> Option<(&str, Vec<&str>)> {
    let parts: Vec<&str> = message.split_whitespace().collect();
    let command_str = parts.first()?;
    let first = *command_str;
    let args = parts[1..].to_vec();
    Some((first, args))
}
