use crate::{
    DiscordClient,
    discord::{AppCommand, AppEvent, VoiceConnectionStatus},
    logging,
};

pub(super) async fn handle(client: DiscordClient, command: AppCommand) {
    match command {
        AppCommand::JoinVoiceChannel {
            guild_id,
            channel_id,
            self_mute,
            self_deaf,
            allow_microphone_transmit,
            microphone_sensitivity,
            microphone_volume,
            voice_output_volume,
        } => {
            if let Err(message) =
                client.update_voice_state(guild_id, Some(channel_id), self_mute, self_deaf)
            {
                logging::error("app", &message);
                client
                    .publish_event(AppEvent::VoiceConnectionStatusChanged {
                        guild_id,
                        channel_id: Some(channel_id),
                        status: VoiceConnectionStatus::Failed,
                        message: Some(message),
                    })
                    .await;
            } else {
                client.update_voice_capture_permission(
                    guild_id,
                    channel_id,
                    allow_microphone_transmit,
                    microphone_sensitivity,
                    microphone_volume,
                    voice_output_volume,
                );
                client
                    .publish_event(AppEvent::VoiceConnectionStatusChanged {
                        guild_id,
                        channel_id: Some(channel_id),
                        status: VoiceConnectionStatus::Connecting,
                        message: Some("Voice join requested".to_owned()),
                    })
                    .await;
            }
        }
        AppCommand::UpdateVoiceState {
            guild_id,
            channel_id,
            self_mute,
            self_deaf,
        } => {
            if let Err(message) =
                client.update_voice_state(guild_id, Some(channel_id), self_mute, self_deaf)
            {
                logging::error("app", &message);
                client
                    .publish_event(AppEvent::GatewayError { message })
                    .await;
            }
        }
        AppCommand::UpdateVoiceCapturePermission {
            guild_id,
            channel_id,
            allow_microphone_transmit,
            microphone_sensitivity,
            microphone_volume,
            voice_output_volume,
        } => {
            client.update_voice_capture_permission(
                guild_id,
                channel_id,
                allow_microphone_transmit,
                microphone_sensitivity,
                microphone_volume,
                voice_output_volume,
            );
        }
        AppCommand::LeaveVoiceChannel {
            guild_id,
            self_mute,
            self_deaf,
        } => {
            if let Err(message) = client.update_voice_state(guild_id, None, self_mute, self_deaf) {
                logging::error("app", &message);
                client
                    .publish_event(AppEvent::VoiceConnectionStatusChanged {
                        guild_id,
                        channel_id: None,
                        status: VoiceConnectionStatus::Failed,
                        message: Some(message),
                    })
                    .await;
            } else {
                client
                    .publish_event(AppEvent::VoiceConnectionStatusChanged {
                        guild_id,
                        channel_id: None,
                        status: VoiceConnectionStatus::Disconnected,
                        message: Some("Voice leave requested".to_owned()),
                    })
                    .await;
            }
        }
        _ => unreachable!("non-voice command routed to voice handler"),
    }
}
