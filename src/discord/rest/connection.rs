use reqwest::header::AUTHORIZATION;

use crate::{AppError, Result};

use super::DiscordRest;

impl DiscordRest {
    /// Fire a cheap REST call to establish the HTTPS connection up front.
    /// `reqwest::Client` lazily opens a TCP+TLS+HTTP/2 connection on the first
    /// request, which costs ~500ms-1s of round-trips. The first user-facing
    /// fetch (e.g. opening a forum) would otherwise pay that cost on top of
    /// the search index cold-start, doubled because we issue two parallel
    /// search calls. Priming the pool at startup lets the first real request
    /// reuse the warmed connection and start in single-digit milliseconds.
    pub async fn prime_connection_pool(&self) -> Result<()> {
        self.raw_http
            .get("https://discord.com/api/v9/users/@me")
            .header(AUTHORIZATION, &self.token)
            .send()
            .await
            .map_err(|error| {
                AppError::DiscordRequest(format!("connection prime request failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::DiscordRequest(format!("connection prime failed: {error}"))
            })?;
        Ok(())
    }
}
