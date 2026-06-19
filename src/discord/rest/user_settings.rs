use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde_json::{Value, json};

use crate::{
    Result,
    discord::{GuildFolder, PresenceStatus},
};

use super::DiscordRest;

impl DiscordRest {
    pub async fn update_guild_folders(&self, folders: &[GuildFolder]) -> Result<()> {
        self.send_unit(
            self.raw_http
                .patch("https://discord.com/api/v9/users/@me/settings-proto/1")
                .json(&settings_proto_request_body(folders)),
            "guild folders settings update",
        )
        .await
    }
}

pub(super) fn settings_proto_request_body(folders: &[GuildFolder]) -> Value {
    json!({
        "settings": BASE64_STANDARD.encode(encode_preloaded_guild_folders(folders)),
    })
}

pub(super) fn status_settings_proto_request_body(status: PresenceStatus) -> Value {
    json!({
        "settings": BASE64_STANDARD.encode(encode_preloaded_status(status)),
    })
}

fn encode_preloaded_status(status: PresenceStatus) -> Vec<u8> {
    let mut status_settings = Vec::new();
    write_len_field(
        &mut status_settings,
        1,
        &encode_string_wrapper(status.gateway_status()),
    );

    let mut settings = Vec::new();
    write_len_field(&mut settings, 11, &status_settings);
    settings
}

fn encode_preloaded_guild_folders(folders: &[GuildFolder]) -> Vec<u8> {
    let mut guild_folders = Vec::new();
    for folder in folders {
        write_len_field(&mut guild_folders, 1, &encode_guild_folder(folder));
    }

    let mut settings = Vec::new();
    write_len_field(&mut settings, 14, &guild_folders);
    settings
}

fn encode_guild_folder(folder: &GuildFolder) -> Vec<u8> {
    let mut bytes = Vec::new();
    if !folder.guild_ids.is_empty() {
        let mut guild_ids = Vec::with_capacity(folder.guild_ids.len() * 8);
        for guild_id in &folder.guild_ids {
            guild_ids.extend_from_slice(&guild_id.get().to_le_bytes());
        }
        write_len_field(&mut bytes, 1, &guild_ids);
    }

    if let Some(id) = folder.id {
        write_len_field(&mut bytes, 2, &encode_varint_wrapper(id));
    }

    if let Some(name) = &folder.name {
        write_len_field(&mut bytes, 3, &encode_string_wrapper(name));
    }

    if let Some(color) = folder.color {
        write_len_field(&mut bytes, 4, &encode_varint_wrapper(u64::from(color)));
    }
    bytes
}

fn encode_varint_wrapper(value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_varint_field(&mut bytes, 1, value);
    bytes
}

fn encode_string_wrapper(value: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_len_field(&mut bytes, 1, value.as_bytes());
    bytes
}

fn write_varint_field(bytes: &mut Vec<u8>, field: u32, value: u64) {
    write_varint(bytes, u64::from(field << 3));
    write_varint(bytes, value);
}

fn write_len_field(bytes: &mut Vec<u8>, field: u32, value: &[u8]) {
    write_varint(bytes, u64::from((field << 3) | 2));
    write_varint(bytes, value.len() as u64);
    bytes.extend_from_slice(value);
}

fn write_varint(bytes: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        bytes.push((value as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}
