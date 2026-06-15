use reqwest::header::AUTHORIZATION;

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, MessageMarker},
};
use crate::{AppError, Result};

use super::DiscordRest;

impl DiscordRest {
    pub async fn vote_poll(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        answer_ids: &[u8],
    ) -> Result<()> {
        let url = format!(
            "https://discord.com/api/v9/channels/{}/polls/{}/answers/@me",
            channel_id.get(),
            message_id.get()
        );
        self.raw_http
            .put(url)
            .header(AUTHORIZATION, &self.token)
            .json(&poll_vote_request_body(answer_ids))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("poll vote request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("poll vote failed: {error}")))?;
        Ok(())
    }
}

pub(super) fn poll_vote_request_body(answer_ids: &[u8]) -> serde_json::Value {
    serde_json::json!({ "answer_ids": answer_ids })
}
