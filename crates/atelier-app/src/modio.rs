//! mod.io: the user's own API key, token sign-in, browsing and social actions.

use std::collections::HashMap;
use std::sync::Arc;

use atelier_core::{GameId, HealthCheck, Severity};
use atelier_modio::{ModDetail, ModioClient, ModioCollection, ModioPage, ModioQuery, ReportKind};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorCode, INVALID_KEY_MESSAGE, NO_API_KEY_MESSAGE, Result};
use crate::{App, build_client, current_key};

const API_KEY_LENGTH: usize = 32;
/// mod.io rates 0..1; the UI shows a whole percentage.
const RATING_SCALE: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// A one-time code mod.io emails, asked for with `request_sign_in_code`.
    Email,
    Token,
}

/// What the UI knows about mod.io access. Never carries the key itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppAccount {
    pub has_api_key: bool,
    pub signed_in: bool,
    pub username: Option<String>,
    pub expires_at: Option<String>,
    pub terms_accepted: bool,
}

impl App {
    pub(crate) fn client(&self) -> Arc<ModioClient> {
        Arc::clone(
            &self
                .0
                .client
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn has_api_key(&self) -> Result<bool> {
        Ok(current_key(&*self.0.api_key_store, self.0.dev_api_key.as_deref())?.is_some())
    }

    pub(crate) fn has_token(&self) -> Result<bool> {
        Ok(self.0.token_store.load()?.is_some())
    }

    /// Browsing needs the user's key or a signed-in token; say so before any request.
    fn browsing_client(&self) -> Result<Arc<ModioClient>> {
        if self.has_api_key()? || self.has_token()? {
            Ok(self.client())
        } else {
            Err(AppError::new(ErrorCode::NoApiKey, NO_API_KEY_MESSAGE))
        }
    }

    fn rebuild_client(&self) -> Result<()> {
        let key = current_key(&*self.0.api_key_store, self.0.dev_api_key.as_deref())?;
        let client = build_client(key, self.0.modio_base.clone(), &self.0.token_store)?;
        *self
            .0
            .client
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(client);
        Ok(())
    }

    /// Checks the key against mod.io before saving it, so a typo never lands in the keychain.
    pub async fn set_api_key(&self, key: &str) -> Result<()> {
        let key = key.trim();
        let well_formed =
            key.len() == API_KEY_LENGTH && key.chars().all(|char| char.is_ascii_hexdigit());
        if !well_formed {
            return Err(AppError::new(ErrorCode::InvalidApiKey, INVALID_KEY_MESSAGE));
        }
        let probe = ModioClient::new(atelier_modio::ClientConfig {
            api_key: Some(key.to_owned()),
            base_override: self.0.modio_base.clone(),
        })?;
        match probe.terms().await {
            Ok(_) => {}
            Err(atelier_modio::Error::Unauthorized) => {
                return Err(AppError::new(ErrorCode::InvalidApiKey, INVALID_KEY_MESSAGE));
            }
            Err(other) => return Err(other.into()),
        }
        self.0.api_key_store.save(key)?;
        self.rebuild_client()
    }

    pub fn clear_api_key(&self) -> Result<()> {
        self.0.api_key_store.clear()?;
        self.rebuild_client()
    }

    /// Offline keeps the stored sign-in; an expired token is forgotten.
    pub async fn account(&self) -> Result<AppAccount> {
        let has_api_key = self.has_api_key()?;
        let terms_accepted = self.persisted().terms_accepted;
        let account = match self.client().account().await {
            Ok(account) => account,
            Err(atelier_modio::Error::Unauthorized) => {
                self.0.token_store.clear()?;
                self.client().account().await?
            }
            Err(atelier_modio::Error::Offline) => atelier_modio::Account {
                signed_in: self.has_token()?,
                username: None,
                expires_at: None,
                terms_accepted,
            },
            Err(other) => return Err(other.into()),
        };
        Ok(AppAccount {
            has_api_key,
            signed_in: account.signed_in,
            username: account.username,
            expires_at: account.expires_at,
            terms_accepted: terms_accepted || account.terms_accepted,
        })
    }

    pub fn accept_terms(&self) -> Result<()> {
        self.update_persisted(|state| state.terms_accepted = true)
    }

    /// Has mod.io email a sign-in code; uses the user's own key.
    pub async fn request_sign_in_code(&self, email: &str) -> Result<()> {
        self.require_terms()?;
        Ok(self.client().request_email_code(email).await?)
    }

    /// `secret` is the emailed code for `Email`, or a pasted personal token for `Token`.
    pub async fn sign_in(&self, method: AuthMethod, secret: Option<&str>) -> Result<()> {
        let Some(secret) = secret.filter(|secret| !secret.trim().is_empty()) else {
            let ask = match method {
                AuthMethod::Email => "Enter the code mod.io emailed you.",
                AuthMethod::Token => "Paste a mod.io access token to sign in.",
            };
            return Err(AppError::new(ErrorCode::InvalidInput, ask));
        };
        self.require_terms()?;
        let client = self.client();
        let signed_in = match method {
            AuthMethod::Email => client.sign_in_with_email_code(secret).await,
            AuthMethod::Token => client.sign_in_with_token(secret).await,
        };
        match signed_in {
            Ok(_) => Ok(()),
            Err(atelier_modio::Error::Unauthorized) => Err(AppError::new(
                ErrorCode::Unauthorized,
                "mod.io rejected that token. Create a read+write token at mod.io/me/access.",
            )),
            Err(other) => Err(other.into()),
        }
    }

    fn require_terms(&self) -> Result<()> {
        if self.persisted().terms_accepted {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::TermsRequired,
                "Accept mod.io's terms first.",
            ))
        }
    }

    pub fn sign_out(&self) -> Result<()> {
        Ok(self.client().sign_out()?)
    }

    pub async fn browse(&self, query: &ModioQuery) -> Result<ModioPage> {
        let mut page = self.browsing_client()?.browse(query).await?;
        let installed = self.installed_by_modio_id(query.game)?;
        for item in &mut page.items {
            item.rating = (item.rating * RATING_SCALE).round();
            item.installed_mod_id = installed.get(&item.id).cloned();
        }
        Ok(page)
    }

    pub async fn modio_tags(&self, game: GameId) -> Result<Vec<String>> {
        Ok(self.browsing_client()?.tags(game).await?)
    }

    pub async fn modio_detail(&self, game: GameId, modio_id: u64) -> Result<ModDetail> {
        let mut detail = self.browsing_client()?.mod_detail(game, modio_id).await?;
        detail.info.rating = (detail.info.rating * RATING_SCALE).round();
        detail.info.installed_mod_id = self.installed_by_modio_id(game)?.remove(&modio_id);
        Ok(detail)
    }

    pub async fn list_collections(&self, game: GameId) -> Result<Vec<ModioCollection>> {
        Ok(self.browsing_client()?.collections(game).await?)
    }

    pub async fn follow_collection(&self, game: GameId, id: u64, follow: bool) -> Result<()> {
        Ok(self.client().follow(game, id, follow).await?)
    }

    pub async fn rate_mod(&self, game: GameId, modio_id: u64, up: bool) -> Result<()> {
        Ok(self.client().rate(game, modio_id, up).await?)
    }

    pub async fn report_mod(
        &self,
        game: GameId,
        modio_id: u64,
        kind: ReportKind,
        summary: &str,
    ) -> Result<()> {
        if summary.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Say what's wrong so mod.io can act on the report.",
            ));
        }
        Ok(self
            .browsing_client()?
            .report(game, modio_id, kind, summary.trim())
            .await?)
    }

    /// Any answer, even a refusal, means mod.io is up; only a network failure is offline.
    pub async fn modio_reachable(&self) -> bool {
        !matches!(
            self.client().terms().await,
            Err(atelier_modio::Error::Offline)
        )
    }

    /// Local checks plus whether mod.io answers and who is signed in.
    pub async fn health_checks(&self, game: GameId) -> Result<Vec<HealthCheck>> {
        let mut checks = self.local_health_checks(game)?;
        checks.push(if self.modio_reachable().await {
            check("modio-reachable", Severity::Ok, "mod.io is reachable", "")
        } else {
            check(
                "modio-reachable",
                Severity::Error,
                "mod.io is unreachable",
                "Installed mods still work. Browsing, subscriptions and updates come back when you reconnect.",
            )
        });
        let account = self.account().await?;
        checks.push(match account.username {
            Some(name) if account.signed_in => check(
                "modio-account",
                Severity::Ok,
                "Signed in to mod.io",
                &format!("{name} · token in the system keychain"),
            ),
            _ => check(
                "modio-account",
                Severity::Warning,
                "Not signed in to mod.io",
                "Browsing and installing work with your API key. Sign in to subscribe, rate and sync.",
            ),
        });
        Ok(checks)
    }

    /// Local mod id per mod.io id, for "installed" badges on mod.io listings.
    fn installed_by_modio_id(&self, game: GameId) -> Result<HashMap<u64, String>> {
        Ok(self
            .list_mods(game)?
            .into_iter()
            .filter_map(|item| item.modio_id.map(|modio_id| (modio_id, item.id)))
            .collect())
    }
}

fn check(id: &str, level: Severity, title: &str, detail: &str) -> HealthCheck {
    HealthCheck {
        id: id.to_owned(),
        level,
        title: title.to_owned(),
        detail: detail.to_owned(),
    }
}
