use crate::{
    DiscordClient,
    discord::{AppCommand, AppEvent},
    logging,
};

const MENTION_MEMBER_SEARCH_LIMIT: u16 = 10;

pub(super) async fn handle(client: DiscordClient, command: AppCommand) {
    match command {
        AppCommand::LoadGuildMembers { guild_id } => {
            if let Err(message) = client.request_guild_members(guild_id) {
                publish_gateway_error(&client, message).await;
            }
        }
        AppCommand::LoadGuildMembersByIds { guild_id, user_ids } => {
            if let Err(message) = client.request_guild_members_by_ids(guild_id, user_ids) {
                publish_gateway_error(&client, message).await;
            }
        }
        AppCommand::SearchGuildMembers { guild_id, query } => {
            if let Err(message) =
                client.search_guild_members(guild_id, query, MENTION_MEMBER_SEARCH_LIMIT)
            {
                publish_gateway_error(&client, message).await;
            }
        }
        AppCommand::SetSelectedGuild { guild_id } => {
            client
                .publish_event(AppEvent::SelectedGuildChanged { guild_id })
                .await;
        }
        AppCommand::SetSelectedMessageChannel { channel_id } => {
            client
                .publish_event(AppEvent::SelectedMessageChannelChanged { channel_id })
                .await;
        }
        AppCommand::SubscribeDirectMessage { channel_id } => {
            if let Err(message) = client.subscribe_direct_message(channel_id) {
                publish_gateway_error(&client, message).await;
            }
        }
        AppCommand::SubscribeGuildChannel {
            guild_id,
            channel_id,
        } => {
            if let Err(message) = client.subscribe_guild_channel(guild_id, channel_id) {
                publish_gateway_error(&client, message).await;
            }
        }
        AppCommand::UpdateMemberListSubscription {
            guild_id,
            channel_id,
            ranges,
        } => {
            if let Err(message) =
                client.update_member_list_subscription(guild_id, channel_id, ranges)
            {
                publish_gateway_error(&client, message).await;
            }
        }
        _ => unreachable!("non-gateway command routed to gateway handler"),
    }
}

async fn publish_gateway_error(client: &DiscordClient, message: String) {
    logging::error("app", &message);
    client
        .publish_event(AppEvent::GatewayError { message })
        .await;
}
