use reqwest::{
    header::AUTHORIZATION,
    multipart::{Form, Part},
};
use serde_json::{Value, json};

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, MessageMarker},
};
use crate::{
    AppError, Result,
    discord::{
        MAX_UPLOAD_ATTACHMENT_COUNT, MAX_UPLOAD_FILE_BYTES, MAX_UPLOAD_TOTAL_BYTES,
        MessageAttachmentUpload, MessageInfo, gateway::parse_message_info,
    },
};

use super::DiscordRest;

impl DiscordRest {
    pub async fn send_message(
        &self,
        channel_id: Id<ChannelMarker>,
        content: &str,
        reply_to: Option<Id<MessageMarker>>,
        attachments: &[MessageAttachmentUpload],
    ) -> Result<MessageInfo> {
        validate_message_payload(content, attachments)?;
        let body = message_request_body(content, reply_to, attachments);

        self.send_message_body(channel_id, body, attachments).await
    }

    pub async fn send_tts_message(
        &self,
        channel_id: Id<ChannelMarker>,
        content: &str,
    ) -> Result<MessageInfo> {
        validate_message_content(content)?;
        let body = message_request_body_with_tts(content, None, &[], true);

        self.send_message_body(channel_id, body, &[]).await
    }

    async fn send_message_body(
        &self,
        channel_id: Id<ChannelMarker>,
        body: Value,
        attachments: &[MessageAttachmentUpload],
    ) -> Result<MessageInfo> {
        let request = self
            .raw_http
            .post(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                channel_id.get()
            ))
            .header(AUTHORIZATION, &self.token);

        let request = if attachments.is_empty() {
            request.json(&body)
        } else {
            request.multipart(message_multipart_form(body, attachments).await?)
        };

        let raw = request
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("send message request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("send message failed: {error}")))?
            .json::<Value>()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("send message decode failed: {error}"))
            })?;
        parse_message_info(&raw).ok_or_else(|| {
            AppError::DiscordRequest("send message response was missing required fields".to_owned())
        })
    }

    pub async fn edit_message(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        content: &str,
    ) -> Result<MessageInfo> {
        validate_message_content(content)?;
        let raw = self
            .raw_http
            .patch(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}",
                channel_id.get(),
                message_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("edit message request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("edit message failed: {error}")))?
            .json::<Value>()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("edit message decode failed: {error}"))
            })?;
        parse_message_info(&raw).ok_or_else(|| {
            AppError::DiscordRequest("edit message response was missing required fields".to_owned())
        })
    }

    pub async fn delete_message(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<()> {
        self.raw_http
            .delete(format!(
                "https://discord.com/api/v9/channels/{}/messages/{}",
                channel_id.get(),
                message_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("delete message request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("delete message failed: {error}")))?;
        Ok(())
    }

    pub async fn load_message_history(
        &self,
        channel_id: Id<ChannelMarker>,
        before: Option<Id<MessageMarker>>,
        limit: u16,
    ) -> Result<Vec<MessageInfo>> {
        let mut request = self
            .raw_http
            .get(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                channel_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .query(&[("limit", limit.to_string())]);
        if let Some(message_id) = before {
            request = request.query(&[("before", message_id.to_string())]);
        }
        let raw_messages: Vec<Value> = request
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("message history request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("message history failed: {error}")))?
            .json()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("message history decode failed: {error}"))
            })?;

        raw_messages
            .iter()
            .map(|raw| {
                parse_message_info(raw).ok_or_else(|| {
                    AppError::DiscordRequest(
                        "history message response was missing required fields".to_owned(),
                    )
                })
            })
            .collect()
    }

    pub async fn load_message_history_around(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        limit: u16,
    ) -> Result<Vec<MessageInfo>> {
        self.load_message_history_with_anchor(channel_id, "around", message_id, limit)
            .await
    }

    pub async fn load_message_history_after(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        limit: u16,
    ) -> Result<Vec<MessageInfo>> {
        self.load_message_history_with_anchor(channel_id, "after", message_id, limit)
            .await
    }

    async fn load_message_history_with_anchor(
        &self,
        channel_id: Id<ChannelMarker>,
        anchor_name: &str,
        message_id: Id<MessageMarker>,
        limit: u16,
    ) -> Result<Vec<MessageInfo>> {
        let raw_messages: Vec<Value> = self
            .raw_http
            .get(format!(
                "https://discord.com/api/v9/channels/{}/messages",
                channel_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .query(&[("limit", limit.to_string())])
            .query(&[(anchor_name, message_id.to_string())])
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("message history request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("message history failed: {error}")))?
            .json()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("message history decode failed: {error}"))
            })?;

        raw_messages
            .iter()
            .map(|raw| {
                parse_message_info(raw).ok_or_else(|| {
                    AppError::DiscordRequest(
                        "history message response was missing required fields".to_owned(),
                    )
                })
            })
            .collect()
    }

    pub async fn load_pinned_messages(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<Vec<MessageInfo>> {
        let raw: Value = self
            .raw_http
            .get(format!(
                "https://discord.com/api/v9/channels/{}/messages/pins",
                channel_id.get()
            ))
            .header(AUTHORIZATION, &self.token)
            .query(&[("limit", "50")])
            .send()
            .await
            .map_err(|error| AppError::DiscordRequest(format!("pins request failed: {error}")))?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("pins failed: {error}")))?
            .json()
            .await
            .map_err(|error| AppError::DiscordRequest(format!("pins decode failed: {error}")))?;
        let messages: Vec<&Value> = match &raw {
            Value::Array(items) => items.iter().collect(),
            Value::Object(object) => object
                .get("items")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("message"))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        messages
            .into_iter()
            .map(|raw| {
                parse_message_info(raw).ok_or_else(|| {
                    AppError::DiscordRequest("pin message was missing required fields".to_owned())
                })
            })
            .collect()
    }

    pub async fn set_message_pinned(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        pinned: bool,
    ) -> Result<()> {
        let request = if pinned {
            self.raw_http.put(format!(
                "https://discord.com/api/v9/channels/{}/pins/{}",
                channel_id.get(),
                message_id.get()
            ))
        } else {
            self.raw_http.delete(format!(
                "https://discord.com/api/v9/channels/{}/pins/{}",
                channel_id.get(),
                message_id.get()
            ))
        };
        request
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| AppError::DiscordRequest(format!("pin request failed: {error}")))?
            .error_for_status()
            .map_err(|error| AppError::DiscordRequest(format!("pin update failed: {error}")))?;
        Ok(())
    }
}

pub(super) fn message_request_body(
    content: &str,
    reply_to: Option<Id<MessageMarker>>,
    attachments: &[MessageAttachmentUpload],
) -> Value {
    message_request_body_with_tts(content, reply_to, attachments, false)
}

pub(super) fn message_request_body_with_tts(
    content: &str,
    reply_to: Option<Id<MessageMarker>>,
    attachments: &[MessageAttachmentUpload],
    tts: bool,
) -> Value {
    let mut body = json!({ "content": content });
    if tts {
        body["tts"] = Value::Bool(true);
    }
    if let Some(message_id) = reply_to {
        body["message_reference"] = json!({ "message_id": message_id.to_string() });
    }
    if !attachments.is_empty() {
        body["attachments"] = Value::Array(
            attachments
                .iter()
                .enumerate()
                .map(|(index, attachment)| {
                    json!({
                        "id": index,
                        "filename": attachment.filename,
                    })
                })
                .collect(),
        );
    }
    body
}

pub(super) async fn message_multipart_form(
    body: Value,
    attachments: &[MessageAttachmentUpload],
) -> Result<Form> {
    let actual_sizes = attachment_sizes(attachments).await?;
    validate_attachment_sizes(&actual_sizes)?;

    let mut form = Form::new().part(
        "payload_json",
        Part::text(body.to_string())
            .mime_str("application/json")
            .map_err(|error| AppError::DiscordRequest(format!("upload payload failed: {error}")))?,
    );

    for (index, attachment) in attachments.iter().enumerate() {
        let bytes = attachment_bytes(attachment).await?;
        validate_attachment_sizes(&[(attachment.filename.clone(), bytes.len() as u64)])?;
        let content_type = upload_content_type(&attachment.filename);
        let part = Part::bytes(bytes)
            .file_name(attachment.filename.clone())
            .mime_str(&content_type)
            .map_err(|error| {
                AppError::DiscordRequest(format!(
                    "attachment {} content type failed: {error}",
                    attachment.filename
                ))
            })?;
        form = form.part(format!("files[{index}]"), part);
    }
    Ok(form)
}

async fn attachment_sizes(attachments: &[MessageAttachmentUpload]) -> Result<Vec<(String, u64)>> {
    let mut sizes = Vec::with_capacity(attachments.len());
    for attachment in attachments {
        let size = if let Some(path) = attachment.path() {
            tokio::fs::metadata(path)
                .await
                .map_err(|error| {
                    AppError::DiscordRequest(format!(
                        "stat attachment {} failed: {error}",
                        attachment.filename
                    ))
                })?
                .len()
        } else {
            attachment.size_bytes
        };
        sizes.push((attachment.filename.clone(), size));
    }
    Ok(sizes)
}

async fn attachment_bytes(attachment: &MessageAttachmentUpload) -> Result<Vec<u8>> {
    if let Some(bytes) = attachment.bytes() {
        return Ok(bytes.to_vec());
    }
    let Some(path) = attachment.path() else {
        return Err(AppError::DiscordRequest(format!(
            "attachment {} has no data",
            attachment.filename
        )));
    };
    tokio::fs::read(path).await.map_err(|error| {
        AppError::DiscordRequest(format!(
            "read attachment {} failed: {error}",
            attachment.filename
        ))
    })
}

pub(super) fn upload_content_type(filename: &str) -> String {
    mime_guess::from_path(filename)
        .first_or_octet_stream()
        .essence_str()
        .to_owned()
}

pub(super) fn validate_message_payload(
    content: &str,
    attachments: &[MessageAttachmentUpload],
) -> Result<()> {
    if content.trim().is_empty() && attachments.is_empty() {
        return Err(AppError::EmptyMessageContent);
    }

    let len = content.chars().count();
    if len > 2_000 {
        return Err(AppError::MessageTooLong { len });
    }

    let sizes = attachments
        .iter()
        .map(|attachment| (attachment.filename.clone(), attachment.size_bytes))
        .collect::<Vec<_>>();
    validate_attachment_sizes(&sizes)
}

fn validate_attachment_sizes(attachments: &[(String, u64)]) -> Result<()> {
    if attachments.len() > MAX_UPLOAD_ATTACHMENT_COUNT {
        return Err(AppError::TooManyAttachments {
            count: attachments.len(),
        });
    }

    let mut total_size = 0_u64;
    for (filename, size) in attachments {
        if *size > MAX_UPLOAD_FILE_BYTES {
            return Err(AppError::AttachmentTooLarge {
                filename: filename.clone(),
                size: *size,
            });
        }
        total_size = total_size.saturating_add(*size);
    }
    if total_size > MAX_UPLOAD_TOTAL_BYTES {
        return Err(AppError::AttachmentsTooLarge { size: total_size });
    }

    Ok(())
}

pub(super) fn validate_message_content(content: &str) -> Result<()> {
    validate_message_payload(content, &[])
}
