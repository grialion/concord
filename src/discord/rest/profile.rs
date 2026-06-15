use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use reqwest::{StatusCode, header::AUTHORIZATION};
use serde_json::{Value, json};

use crate::discord::ids::{
    Id,
    marker::{GuildMarker, RoleMarker, UserMarker},
};
use crate::{
    AppError, Result,
    discord::{
        FriendStatus, GlobalUserProfileUpdate, GuildUserProfileUpdate, MutualGuildInfo,
        UserProfileInfo, UserProfileUpdate, events::avatar_hash_extension,
        read_profile_avatar_image,
    },
};

use super::DiscordRest;

impl DiscordRest {
    pub async fn load_user_profile(
        &self,
        user_id: Id<UserMarker>,
        guild_id: Option<Id<crate::discord::ids::marker::GuildMarker>>,
        is_self: bool,
    ) -> Result<UserProfileInfo> {
        let mut url = format!(
            "https://discord.com/api/v9/users/{}/profile?",
            user_id.get(),
        );
        if !is_self {
            url.push_str("with_mutual_guilds=true&with_mutual_friends_count=true&");
        }
        if let Some(guild_id) = guild_id {
            url.push_str(&format!("guild_id={}", guild_id.get()));
        }
        let response = self
            .raw_http
            .get(url)
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("user profile request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("user profile failed: {error}")))?;
        let body: Value = response.json().await.map_err(|error| {
            AppError::DiscordRequest(format!("user profile decode failed: {error}"))
        })?;

        Ok(parse_user_profile_response(user_id, &body, None))
    }

    /// Returns the user's saved note, or `None` if Discord responds 404
    /// (no note set). Other errors propagate.
    pub(in crate::discord) async fn load_user_note(
        &self,
        user_id: Id<UserMarker>,
    ) -> Result<Option<String>> {
        let url = format!(
            "https://discord.com/api/v9/users/@me/notes/{}",
            user_id.get()
        );
        let response = self
            .raw_http
            .get(url)
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("user note request failed: {error}"))
            })?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("user note failed: {error}")))?;
        let body: Value = response.json().await.map_err(|error| {
            AppError::DiscordRequest(format!("user note decode failed: {error}"))
        })?;
        Ok(body
            .get("note")
            .and_then(Value::as_str)
            .filter(|note| !note.is_empty())
            .map(str::to_owned))
    }

    pub async fn update_user_profile(&self, update: &UserProfileUpdate) -> Result<()> {
        if !update.global.is_empty() {
            self.update_global_user_profile(&update.global).await?;
        }
        if let Some(guild) = update.guild.as_ref().filter(|guild| !guild.is_empty()) {
            self.update_guild_user_profile(guild).await?;
        }
        Ok(())
    }

    async fn update_global_user_profile(&self, update: &GlobalUserProfileUpdate) -> Result<()> {
        let mut user_body = serde_json::Map::new();
        if let Some(display_name) = &update.display_name {
            user_body.insert("global_name".to_owned(), nullable_text_value(display_name));
        }
        if let Some(avatar) = &update.avatar {
            user_body.insert(
                "avatar".to_owned(),
                Value::String(profile_avatar_data_uri(avatar).await?),
            );
        }
        if !user_body.is_empty() {
            self.raw_http
                .patch("https://discord.com/api/v9/users/@me")
                .header(AUTHORIZATION, &self.token)
                .json(&Value::Object(user_body))
                .send()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!(
                        "global profile update request failed: {error}"
                    ))
                })?
                .error_for_status()
                .map_err(|error| {
                    AppError::DiscordRequest(format!("global profile update failed: {error}"))
                })?;
        }
        if let Some(pronouns) = &update.pronouns {
            self.raw_http
                .patch("https://discord.com/api/v9/users/@me/profile")
                .header(AUTHORIZATION, &self.token)
                .json(&json!({ "pronouns": pronouns }))
                .send()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!(
                        "profile pronouns update request failed: {error}"
                    ))
                })?
                .error_for_status()
                .map_err(|error| {
                    AppError::DiscordRequest(format!("profile pronouns update failed: {error}"))
                })?;
        }
        Ok(())
    }

    async fn update_guild_user_profile(&self, update: &GuildUserProfileUpdate) -> Result<()> {
        if let Some(nickname) = &update.nickname {
            self.raw_http
                .patch(format!(
                    "https://discord.com/api/v9/guilds/{}/members/@me",
                    update.guild_id.get()
                ))
                .header(AUTHORIZATION, &self.token)
                .json(&json!({ "nick": nullable_text_value(nickname) }))
                .send()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!(
                        "guild nickname update request failed: {error}"
                    ))
                })?
                .error_for_status()
                .map_err(|error| {
                    AppError::DiscordRequest(format!("guild nickname update failed: {error}"))
                })?;
        }
        if let Some(pronouns) = &update.pronouns {
            self.raw_http
                .patch(format!(
                    "https://discord.com/api/v9/guilds/{}/profile/@me",
                    update.guild_id.get()
                ))
                .header(AUTHORIZATION, &self.token)
                .json(&json!({ "pronouns": pronouns }))
                .send()
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!(
                        "guild profile update request failed: {error}"
                    ))
                })?
                .error_for_status()
                .map_err(|error| {
                    AppError::DiscordRequest(format!("guild profile update failed: {error}"))
                })?;
        }
        Ok(())
    }
}

fn nullable_text_value(value: &str) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        Value::String(value.to_owned())
    }
}

async fn profile_avatar_data_uri(avatar: &crate::discord::ProfileAvatarUpload) -> Result<String> {
    let image = read_profile_avatar_image(avatar)
        .await
        .map_err(AppError::DiscordRequest)?;
    Ok(format!(
        "data:{};base64,{}",
        image.content_type,
        BASE64_STANDARD.encode(image.bytes)
    ))
}

/// Builds the dashboard's `UserProfileInfo` from Discord's
/// `/users/{id}/profile` JSON. Friend status is left as `None` here because the
/// caller fills it in from cached relationship data.
pub(super) fn parse_user_profile_response(
    user_id: Id<UserMarker>,
    body: &Value,
    note: Option<String>,
) -> UserProfileInfo {
    let user = body.get("user");
    let username = user
        .and_then(|user| user.get("username"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let global_name = user
        .and_then(|user| user.get("global_name"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let avatar_url = user.and_then(profile_avatar_url);
    let user_profile = body.get("user_profile");
    let bio = user_profile
        .and_then(|profile| profile.get("bio"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let pronouns = user_profile
        .and_then(|profile| profile.get("pronouns"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let guild_pronouns = body
        .get("guild_member_profile")
        .and_then(|profile| profile.get("pronouns"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let mutual_guilds = body
        .get("mutual_guilds")
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|entry| {
                    let guild_id = entry
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|raw| raw.parse::<u64>().ok())
                        .and_then(Id::<GuildMarker>::new_checked)?;
                    let nick = entry
                        .get("nick")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned);
                    Some(MutualGuildInfo { guild_id, nick })
                })
                .collect()
        })
        .unwrap_or_default();
    let mutual_friends_count = body
        .get("mutual_friends_count")
        .and_then(Value::as_u64)
        .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
        .unwrap_or(0);
    let guild_nick = body
        .get("guild_member")
        .and_then(|member| member.get("nick"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let role_ids = body
        .get("guild_member")
        .and_then(|member| member.get("roles"))
        .and_then(Value::as_array)
        .map(|roles| roles.iter().filter_map(parse_profile_role_id).collect())
        .unwrap_or_default();

    UserProfileInfo {
        user_id,
        username,
        global_name,
        guild_nick,
        role_ids,
        avatar_url,
        bio,
        pronouns,
        guild_pronouns,
        mutual_guilds,
        mutual_friends_count,
        friend_status: FriendStatus::None,
        note,
    }
}

fn parse_profile_role_id(value: &Value) -> Option<Id<RoleMarker>> {
    value
        .as_str()
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| value.as_u64())
        .and_then(Id::new_checked)
}

fn profile_avatar_url(user: &Value) -> Option<String> {
    let user_id = user
        .get("id")
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<u64>().ok())?;
    let hash = user
        .get("avatar")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())?;
    let extension = avatar_hash_extension(hash);
    Some(format!(
        "https://cdn.discordapp.com/avatars/{user_id}/{hash}.{extension}"
    ))
}
