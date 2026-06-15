use reqwest::header::AUTHORIZATION;
use serde_json::Value;

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, MessageMarker, UserMarker},
};
use crate::{
    AppError, Result,
    discord::{ReactionEmoji, ReactionUserInfo},
};

use super::DiscordRest;

const REACTION_USERS_PAGE_LIMIT: u16 = 100;
pub(super) const REACTION_USERS_MAX_PAGES: usize = 3;

impl DiscordRest {
    pub async fn add_reaction(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        emoji: &ReactionEmoji,
    ) -> Result<()> {
        self.raw_http
            .put(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}/reactions/{}/@me",
                channel_id.get(),
                message_id.get(),
                reaction_route_component(emoji)
            ))
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("add reaction request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("add reaction failed: {error}")))?;
        Ok(())
    }

    pub async fn remove_current_user_reaction(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        emoji: &ReactionEmoji,
    ) -> Result<()> {
        self.raw_http
            .delete(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}/reactions/{}/@me",
                channel_id.get(),
                message_id.get(),
                reaction_route_component(emoji)
            ))
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("remove reaction request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::DiscordRequest(format!("remove reaction failed: {error}"))
            })?;
        Ok(())
    }

    pub async fn load_reaction_users(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        emoji: &ReactionEmoji,
    ) -> Result<Vec<ReactionUserInfo>> {
        let mut users = Vec::new();
        let mut after: Option<Id<UserMarker>> = None;
        let mut pages_loaded = 0usize;

        loop {
            let mut request = self
                .raw_http
                .get(format!(
                    "https://discord.com/api/v9/channels/{}/messages/{}/reactions/{}",
                    channel_id.get(),
                    message_id.get(),
                    reaction_route_component(emoji)
                ))
                .header(AUTHORIZATION, &self.token)
                .query(&[
                    ("limit", REACTION_USERS_PAGE_LIMIT.to_string()),
                    ("type", "0".to_owned()),
                ]);
            if let Some(user_id) = after {
                request = request.query(&[("after", user_id.to_string())]);
            }

            let page: Vec<Value> = request
                .send()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!("reaction users request failed: {error}"))
                })?
                .error_for_status()
                .map_err(|error| {
                    AppError::DiscordRequest(format!("reaction users failed: {error}"))
                })?
                .json()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!("reaction users decode failed: {error}"))
                })?;
            let parsed_page: Vec<ReactionUserInfo> = page
                .iter()
                .filter_map(reaction_user_info_from_raw)
                .collect();
            pages_loaded = pages_loaded.saturating_add(1);
            let next_after = next_reaction_users_after(
                parsed_page.len(),
                parsed_page.last().map(|user| user.user_id),
                pages_loaded,
            );
            users.extend(parsed_page);

            let Some(user_id) = next_after else {
                break;
            };
            after = Some(user_id);
        }

        Ok(users)
    }
}

fn reaction_user_info_from_raw(value: &Value) -> Option<ReactionUserInfo> {
    let user_id = value
        .get("id")
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<u64>().ok())
        .and_then(Id::<UserMarker>::new_checked)?;
    let display_name = value
        .get("global_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| value.get("username").and_then(Value::as_str))?
        .to_owned();

    Some(ReactionUserInfo {
        user_id,
        display_name,
    })
}

pub(super) fn reaction_route_component(emoji: &ReactionEmoji) -> String {
    emoji.route_component()
}

pub(super) fn next_reaction_users_after(
    page_len: usize,
    last_user_id: Option<Id<UserMarker>>,
    pages_loaded: usize,
) -> Option<Id<UserMarker>> {
    (pages_loaded < REACTION_USERS_MAX_PAGES && page_len == usize::from(REACTION_USERS_PAGE_LIMIT))
        .then_some(last_user_id)
        .flatten()
}
