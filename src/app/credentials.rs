use crate::{Result, config, discord::validate_token_header, error::AppError, token_store, tui};

pub(super) struct ResolvedToken {
    pub(super) token: String,
    pub(super) warnings: Vec<String>,
}

pub(super) async fn resolve_token() -> Result<ResolvedToken> {
    let mut warnings = Vec::new();
    let credential_store = match config::load_options() {
        Ok(options) => options.credentials.store,
        Err(error) => {
            warnings.push(format!(
                "config could not be loaded for credential settings: {error}; using auto credential storage"
            ));
            config::CredentialStoreMode::default()
        }
    };

    match load_token_from_store(credential_store).await {
        Ok(Some(token)) => {
            if let Err(error) = validate_token_header(&token) {
                warnings.push(format!(
                    "saved Discord token is invalid: {error}; enter a new token"
                ));
            } else {
                return Ok(ResolvedToken { token, warnings });
            }
        }
        Ok(None) => {}
        Err(error) => warnings.push(format!(
            "credential store unavailable: {error}; enter a token to continue for this session"
        )),
    }

    let login_notice = login_notice_for_token_warnings(&warnings);

    let token = tui::prompt_login(login_notice).await?;
    validate_token_header(&token)?;
    match save_token_to_store(token.clone(), credential_store).await {
        Ok(token_store::TokenSaveLocation::PlaintextFile)
            if credential_store == config::CredentialStoreMode::Auto =>
        {
            warnings.push(
                "system keychain is unavailable; token was saved to the plaintext fallback credential store"
                    .to_owned(),
            );
        }
        Ok(_) => {}
        Err(error) => warnings.push(format!("token was not saved: {error}")),
    }

    Ok(ResolvedToken { token, warnings })
}

async fn load_token_from_store(store: config::CredentialStoreMode) -> Result<Option<String>> {
    tokio::task::spawn_blocking(move || token_store::load_token(store))
        .await
        .map_err(|source| AppError::CredentialStoreTask { source })?
}

async fn save_token_to_store(
    token: String,
    store: config::CredentialStoreMode,
) -> Result<token_store::TokenSaveLocation> {
    tokio::task::spawn_blocking(move || token_store::save_token(&token, store))
        .await
        .map_err(|source| AppError::CredentialStoreTask { source })?
}

fn login_notice_for_token_warnings(warnings: &[String]) -> Option<String> {
    if warnings
        .iter()
        .any(|warning| warning.starts_with("saved Discord token"))
    {
        Some("Saved Discord token is invalid; enter a new token.".to_owned())
    } else if warnings.is_empty() {
        None
    } else {
        Some("Credential storage is unavailable; token may not be saved.".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::login_notice_for_token_warnings;

    #[test]
    fn login_notice_for_token_warnings_reports_user_action() {
        let cases = [
            (
                "saved Discord token is invalid: bad; enter a new token",
                "Saved Discord token is invalid; enter a new token.",
            ),
            (
                "credential store unavailable: permission denied",
                "Credential storage is unavailable; token may not be saved.",
            ),
        ];

        for (warning, expected) in cases {
            let warnings = vec![warning.to_owned()];
            assert_eq!(
                login_notice_for_token_warnings(&warnings).as_deref(),
                Some(expected)
            );
        }
    }
}
