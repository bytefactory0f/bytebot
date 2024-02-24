use serde::Deserialize;
use std::collections::HashMap;
use twitch_client_rs::irc::UserContext;

#[derive(Deserialize)]
pub struct CommandConfig {
    pub commands: Vec<Command>,
}

impl From<CommandConfig> for HashMap<String, Command> {
    fn from(value: CommandConfig) -> Self {
        value
            .commands
            .into_iter()
            .map(|c| (c.prompt.clone(), c))
            .collect()
    }
}

#[derive(Deserialize)]
pub enum Role {
    Broadcaster,
    Mod,
    User,
}

#[derive(Deserialize)]
pub struct Command {
    pub prompt: String,
    pub reply: String,
    pub args: Option<Vec<String>>,

    /// Roles that that this command will be executed for
    pub roles: Option<Vec<Role>>,
}

impl Command {
    fn is_permitted(&self, user_context: UserContext) -> bool {
        match &self.roles {
            Some(roles) => roles.iter().any(|role| match role {
                Role::Broadcaster => user_context.is_broadcaster,
                Role::Mod => user_context.is_mod,
                Role::User => true,
            }),
            None => true,
        }
    }

    pub fn get_reply(&self, values: &[&str], user_context: UserContext) -> Option<String> {
        if !self.is_permitted(user_context) {
            return None;
        }

        let mut reply = self.reply.clone();

        match &self.args {
            Some(args) => {
                for (arg, value) in args.iter().zip(values.iter()) {
                    let pattern = format!("{{{arg}}}");
                    reply = reply.replace(pattern.as_str(), value);
                }

                Some(reply)
            }
            None => Some(reply),
        }
    }
}
