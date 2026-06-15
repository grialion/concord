use reqwest::header::AUTHORIZATION;

use crate::discord::ids::{Id, marker::GuildMarker};
use crate::{AppError, Result};

use super::DiscordRest;

impl DiscordRest {
    pub async fn leave_guild(&self, guild_id: Id<GuildMarker>) -> Result<()> {
        self.raw_http
            .delete(format!(
                "https://discord.com/api/v9/users/@me/guilds/{}",
                guild_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("leave guild request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("leave guild failed: {error}")))?;
        Ok(())
    }
}
