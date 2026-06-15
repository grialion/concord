use reqwest::header::AUTHORIZATION;
use serde_json::{Value, json};

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, MessageMarker},
};
use crate::{AppError, Result};

use super::DiscordRest;

impl DiscordRest {
    /// `token: null` is the legacy anti-spam echo field. Modern clients
    /// always send null.
    pub async fn ack_channel(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<()> {
        self.raw_http
            .post(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}/ack",
                channel_id.get(),
                message_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .json(&json!({ "token": Value::Null }))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("ack channel request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("ack channel failed: {error}")))?;
        Ok(())
    }

    pub async fn ack_channels(
        &self,
        targets: &[(Id<ChannelMarker>, Id<MessageMarker>)],
    ) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }

        let read_states: Vec<_> = targets
            .iter()
            .map(|(channel_id, message_id)| {
                json!({
                    "read_state_type": 0,
                    "channel_id": channel_id.get().to_string(),
                    "message_id": message_id.get().to_string(),
                })
            })
            .collect();

        self.raw_http
            .post("https://discord.com/api/v9/read-states/ack-bulk")
            .header(AUTHORIZATION, &self.token)
            .json(&json!({ "read_states": read_states }))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("ack channels request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("ack channels failed: {error}")))?;
        Ok(())
    }
}
