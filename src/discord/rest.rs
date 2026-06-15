use crate::discord::fingerprint::discord_rest_client;

mod application_commands;
mod connection;
mod forum;
mod guilds;
mod messages;
mod notification_settings;
mod polls;
mod presence;
mod profile;
mod reactions;
mod read_state;
mod search;

pub use forum::ForumPostPage;

#[derive(Clone, Debug)]
pub struct DiscordRest {
    raw_http: reqwest::Client,
    token: String,
}

impl DiscordRest {
    pub fn new(token: String) -> Self {
        Self {
            raw_http: discord_rest_client(),
            token,
        }
    }
}

#[cfg(test)]
mod tests;
