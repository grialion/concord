use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::header::AUTHORIZATION;
use serde_json::{Value, json};

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};
use crate::{AppError, Result};

use super::DiscordRest;

impl DiscordRest {
    pub async fn set_guild_muted(
        &self,
        guild_id: Id<GuildMarker>,
        muted: bool,
        mute_end_time: Option<DateTime<Utc>>,
        selected_time_window: Option<i64>,
    ) -> Result<()> {
        self.raw_http
            .patch(format!(
                "https://discord.com/api/v9/users/@me/guilds/{}/settings",
                guild_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .json(&mute_request_body(
                muted,
                mute_end_time,
                selected_time_window,
            ))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("set guild mute request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("set guild mute failed: {error}")))?;
        Ok(())
    }

    pub async fn set_channel_muted(
        &self,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
        muted: bool,
        mute_end_time: Option<DateTime<Utc>>,
        selected_time_window: Option<i64>,
    ) -> Result<()> {
        let endpoint = match guild_id {
            Some(guild_id) => format!(
                "https://discord.com/api/v9/users/@me/guilds/{}/settings",
                guild_id.get()
            ),
            None => "https://discord.com/api/v9/users/@me/guilds/@me/settings".to_owned(),
        };
        self.raw_http
            .patch(endpoint)
            .header(AUTHORIZATION, &self.token)
            .json(&json!({
                "channel_overrides": {
                    channel_id.to_string(): mute_request_body(
                        muted,
                        mute_end_time,
                        selected_time_window,
                    ),
                }
            }))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("set channel mute request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::DiscordRequest(format!("set channel mute failed: {error}"))
            })?;
        Ok(())
    }
}

pub(super) fn mute_request_body(
    muted: bool,
    mute_end_time: Option<DateTime<Utc>>,
    selected_time_window: Option<i64>,
) -> Value {
    json!({
        "muted": muted,
        "mute_config": selected_time_window.map(|selected_time_window| json!({
            "end_time": mute_end_time.map(|end_time| {
                end_time.to_rfc3339_opts(SecondsFormat::Millis, true)
            }),
            "selected_time_window": selected_time_window,
        })),
    })
}
