mod commands;
mod settings;

use commands::CommandConfig;
use config::{Config, Environment, File};
use log::{debug, error, info};
use settings::Settings;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use twitch_client_rs::credentials::Credentials;
use twitch_client_rs::irc::IRCMessage;
use twitch_client_rs::twitch_client::{Capability, TwitchClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let home = env::var("HOME").unwrap_or_default();

    let settings = Config::builder()
        .add_source(File::with_name("/etc/bytebot/secrets.yaml").required(false))
        .add_source(
            File::with_name(format!("{}/.config/bytebot/secrets.yaml", home).as_str())
                .required(false),
        )
        .add_source(File::with_name("./secrets.yaml").required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize::<Settings>()?;

    let command_config = Config::builder()
        .add_source(File::with_name("/etc/bytebot/commands.yaml").required(false))
        .add_source(
            File::with_name(format!("{}/.config/bytebot/commands.yaml", home).as_str())
                .required(false),
        )
        .add_source(File::with_name("./commands.yaml").required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize::<CommandConfig>()?;

    let commands = HashMap::from(command_config);

    let channel = "bytefactory";

    let credentials = Credentials {
        refresh_token: settings.refresh_token,
        client_id: settings.client_id,
        client_secret: settings.client_secret,
    };

    let mut twitch_client = TwitchClient::new(credentials, settings.nick, true);

    // Get/refresh the access token
    twitch_client.update_access_token().await?;
    debug!("refreshed access token");

    // Open a websocket connection to twitch
    twitch_client.connect().await?;
    debug!("established websocket connection");

    // Authenticate with the twitch IRC server
    twitch_client.authenticate().await?;
    debug!("successfully authenticated with the twitch IRC server");

    twitch_client.cap_req(&[Capability::Tags]).await?;

    // Joint a twitch chat
    twitch_client.join(channel).await?;
    info!("Now connected to channel: #{channel}");

    while let Some(irc_message) = twitch_client.next().await {
        match irc_message {
            Ok(message) => {
                if let IRCMessage::Privmsg {
                    message,
                    user_context,
                    ..
                } = message
                {
                    info!("Got message: {}", message);
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
                error!("Error handling message: {}", err);
            }
        }
    }

    info!("Shutting down!");
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
