use crate::{DiscordClient, discord::AppCommand};

use super::command_loop::log_app_error;

pub(super) async fn handle(client: DiscordClient, command: AppCommand) {
    match command {
        AppCommand::AckChannel {
            channel_id,
            message_id,
        } => {
            client.clear_read_ack(channel_id);
            client
                .publish_optimistic_read_ack(channel_id, message_id)
                .await;
            // A failure here only loses cross-client sync because the backend
            // has already published the local read state update.
            if let Err(error) = client.ack_channel(channel_id, message_id).await {
                log_app_error("ack channel failed", &error);
            }
        }
        AppCommand::ScheduleAckChannel {
            channel_id,
            message_id,
        } => {
            client
                .publish_optimistic_read_ack(channel_id, message_id)
                .await;
            client.schedule_read_ack(channel_id, message_id, std::time::Instant::now());
        }
        AppCommand::AckChannels { targets } => {
            client.clear_read_acks(targets.iter().map(|(channel_id, _)| *channel_id));
            client.publish_optimistic_read_acks(&targets).await;
            // A failure here only loses cross-client sync because the backend
            // has already published the local read state updates.
            if let Err(error) = client.ack_channels(&targets).await {
                log_app_error("ack channels failed", &error);
            }
        }
        _ => unreachable!("non-read-state command routed to read-state handler"),
    }
}
