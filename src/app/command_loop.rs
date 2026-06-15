use std::sync::Arc;

use tokio::sync::{Semaphore, mpsc};

use crate::{DiscordClient, discord::AppCommand, error::AppError, logging};

use super::{
    gateway_commands, history_commands, media_commands, message_commands, notification_commands,
    read_state_commands, user_commands, voice_commands,
};

const MAX_CONCURRENT_ATTACHMENT_PREVIEWS: usize = 4;
const MAX_CONCURRENT_ATTACHMENT_DOWNLOADS: usize = 2;

pub(super) fn start_command_loop(
    client: DiscordClient,
    mut commands: mpsc::Receiver<AppCommand>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let attachment_preview_permits =
            Arc::new(Semaphore::new(MAX_CONCURRENT_ATTACHMENT_PREVIEWS));
        let attachment_download_permits =
            Arc::new(Semaphore::new(MAX_CONCURRENT_ATTACHMENT_DOWNLOADS));
        // Spawn commands independently so slow REST calls do not block the
        // whole UI command queue.
        while let Some(command) = commands.recv().await {
            let client = client.clone();
            let attachment_preview_permits = attachment_preview_permits.clone();
            let attachment_download_permits = attachment_download_permits.clone();
            tokio::spawn(async move {
                match command {
                    command @ (AppCommand::LoadMessageHistory { .. }
                    | AppCommand::RefreshMessageHistory { .. }
                    | AppCommand::LoadMessageHistoryAfter { .. }
                    | AppCommand::CatchUpMessageHistoryAfter { .. }
                    | AppCommand::LoadMessageHistoryAround { .. }
                    | AppCommand::LoadThreadPreview { .. }
                    | AppCommand::LoadForumPosts { .. }
                    | AppCommand::SearchMessages { .. }) => {
                        history_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::LoadGuildMembers { .. }
                    | AppCommand::LoadGuildMembersByIds { .. }
                    | AppCommand::SearchGuildMembers { .. }
                    | AppCommand::SetSelectedGuild { .. }
                    | AppCommand::SetSelectedMessageChannel { .. }
                    | AppCommand::SubscribeDirectMessage { .. }
                    | AppCommand::SubscribeGuildChannel { .. }
                    | AppCommand::UpdateMemberListSubscription { .. }) => {
                        gateway_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::JoinVoiceChannel { .. }
                    | AppCommand::UpdateVoiceState { .. }
                    | AppCommand::UpdateVoiceCapturePermission { .. }
                    | AppCommand::LeaveVoiceChannel { .. }) => {
                        voice_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::LoadAttachmentPreview { .. }
                    | AppCommand::LoadProfileAvatarPreview { .. }
                    | AppCommand::OpenUrl { .. }
                    | AppCommand::PlayMedia { .. }
                    | AppCommand::DownloadAttachment { .. }) => {
                        media_commands::handle(
                            client,
                            command,
                            attachment_preview_permits,
                            attachment_download_permits,
                        )
                        .await;
                    }
                    command @ (AppCommand::SendMessage { .. }
                    | AppCommand::SendTtsMessage { .. }
                    | AppCommand::LoadApplicationCommands { .. }
                    | AppCommand::RunApplicationCommand { .. }
                    | AppCommand::EditMessage { .. }
                    | AppCommand::DeleteMessage { .. }
                    | AppCommand::LeaveGuild { .. }
                    | AppCommand::AddReaction { .. }
                    | AppCommand::RemoveReaction { .. }
                    | AppCommand::LoadReactionUsers { .. }
                    | AppCommand::LoadPinnedMessages { .. }
                    | AppCommand::SetMessagePinned { .. }
                    | AppCommand::VotePoll { .. }) => {
                        message_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::LoadUserProfile { .. }
                    | AppCommand::LoadUserNote { .. }
                    | AppCommand::UpdateUserProfile { .. }
                    | AppCommand::UpdateCurrentUserStatus { .. }
                    | AppCommand::UpdateCurrentUserActivity { .. }) => {
                        user_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::AckChannel { .. }
                    | AppCommand::ScheduleAckChannel { .. }
                    | AppCommand::AckChannels { .. }) => {
                        read_state_commands::handle(client, command).await;
                    }
                    command @ (AppCommand::SetGuildMuted { .. }
                    | AppCommand::SetChannelMuted { .. }) => {
                        notification_commands::handle(client, command).await;
                    }
                }
            });
        }
    })
}

pub(super) fn log_app_error(context: &str, error: &AppError) {
    logging::error(
        "app",
        format!("{context}: {}; detail={}", error, error.log_detail()),
    );
}
