use crate::{Result, discord::PresenceStatus};

use super::{DiscordRest, user_settings::status_settings_proto_request_body};

impl DiscordRest {
    pub async fn update_current_user_status(&self, status: PresenceStatus) -> Result<()> {
        self.send_unit(
            self.raw_http
                .patch("https://discord.com/api/v9/users/@me/settings-proto/1")
                .json(&status_settings_proto_request_body(status)),
            "status settings update",
        )
        .await
    }
}
