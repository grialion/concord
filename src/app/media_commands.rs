use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::{
    DiscordClient,
    discord::{AppCommand, AppEvent, MediaPlaybackRequestId, read_profile_avatar_image},
    logging,
};

use super::media_adapters;

pub(super) async fn handle(
    client: DiscordClient,
    command: AppCommand,
    attachment_preview_permits: Arc<Semaphore>,
    attachment_download_permits: Arc<Semaphore>,
) {
    match command {
        AppCommand::LoadAttachmentPreview { url } => {
            let Ok(_permit) = attachment_preview_permits.acquire_owned().await else {
                let message = "attachment preview limiter closed".to_owned();
                logging::error("preview", &message);
                client
                    .publish_event(AppEvent::AttachmentPreviewLoadFailed { url, message })
                    .await;
                return;
            };
            match timeout(
                media_adapters::ATTACHMENT_PREVIEW_TIMEOUT,
                media_adapters::fetch_attachment_preview(&url),
            )
            .await
            {
                Err(_) => {
                    let message = "download image preview timed out".to_owned();
                    logging::error("preview", &message);
                    client
                        .publish_event(AppEvent::AttachmentPreviewLoadFailed { url, message })
                        .await;
                }
                Ok(bytes) => match bytes {
                    Ok(bytes) => {
                        client
                            .publish_event(AppEvent::AttachmentPreviewLoaded { url, bytes })
                            .await
                    }
                    Err(message) => {
                        logging::error("preview", &message);
                        client
                            .publish_event(AppEvent::AttachmentPreviewLoadFailed { url, message })
                            .await;
                    }
                },
            }
        }
        AppCommand::LoadProfileAvatarPreview { key, upload } => {
            match read_profile_avatar_image(&upload).await {
                Ok(image) => {
                    client
                        .publish_event(AppEvent::AttachmentPreviewLoaded {
                            url: key,
                            bytes: image.bytes,
                        })
                        .await;
                }
                Err(message) => {
                    logging::error("preview", &message);
                    client
                        .publish_event(AppEvent::AttachmentPreviewLoadFailed { url: key, message })
                        .await;
                }
            }
        }
        AppCommand::OpenUrl { url } => {
            if let Err(error) = media_adapters::open_url(&url) {
                logging::error("app", format!("open url failed: {error}"));
                client
                    .publish_event(AppEvent::GatewayError {
                        message: format!("open url failed: {error}"),
                    })
                    .await;
            }
        }
        AppCommand::PlayMedia { target, request_id } => {
            let request_id = request_id.unwrap_or_else(|| MediaPlaybackRequestId::new(0));
            if let Err(error) =
                media_adapters::play_media(client.clone(), request_id, &target.url, &target.label)
                    .await
            {
                logging::error("media", format!("play media failed: {error}"));
                let label = if target.label.is_empty() {
                    "media"
                } else {
                    target.label.as_str()
                };
                client
                    .publish_event(AppEvent::GatewayError {
                        message: format!("play {label} failed: {error}"),
                    })
                    .await;
            }
        }
        AppCommand::DownloadAttachment {
            id,
            url,
            filename,
            source,
        } => {
            let Ok(_permit) = attachment_download_permits.acquire_owned().await else {
                let message = "attachment download limiter closed".to_owned();
                logging::error("attachment", &message);
                client
                    .publish_event(AppEvent::AttachmentDownloadFailed {
                        id,
                        filename,
                        message,
                        source,
                    })
                    .await;
                return;
            };
            match media_adapters::download_attachment(&client, id, &url, &filename, source).await {
                Ok(path) => {
                    client
                        .publish_event(AppEvent::AttachmentDownloadCompleted {
                            id,
                            path: path.display().to_string(),
                            source,
                        })
                        .await
                }
                Err(message) => {
                    logging::error("attachment", &message);
                    client
                        .publish_event(AppEvent::AttachmentDownloadFailed {
                            id,
                            filename,
                            message,
                            source,
                        })
                        .await;
                }
            }
        }
        _ => unreachable!("non-media command routed to media handler"),
    }
}
