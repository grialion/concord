use reqwest::header::AUTHORIZATION;
use serde_json::json;

use crate::{AppError, Result, discord::PresenceStatus};

use super::DiscordRest;

impl DiscordRest {
    pub async fn update_current_user_status(&self, status: PresenceStatus) -> Result<()> {
        self.raw_http
            .patch("https://discord.com/api/v9/users/@me/settings")
            .header(AUTHORIZATION, &self.token)
            .json(&json!({ "status": status.gateway_status() }))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("status settings update request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::DiscordRequest(format!("status settings update failed: {error}"))
            })?;
        Ok(())
    }
}
