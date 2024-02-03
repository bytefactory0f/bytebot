use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub access_token: String,
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub nick: String,
}
