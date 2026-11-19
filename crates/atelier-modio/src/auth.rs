//! Token sign-in. Email-code, device and OAuth flows await mod.io approval.

use atelier_core::GameId;

use crate::api::{ApiTerms, ApiUser};
use crate::client::{Auth, ModioClient};
use crate::error::{Error, Result};
use crate::types::{Account, Terms};

/// Auth endpoints are game-agnostic; any game host serves them.
const AUTH_GAME: GameId = GameId::Anno117;

impl ModioClient {
    /// The terms the user must accept before any login.
    pub async fn terms(&self) -> Result<Terms> {
        let terms: ApiTerms = self
            .get_json(AUTH_GAME, "authenticate/terms", Auth::Key, &[])
            .await?;
        Ok(terms.into())
    }

    /// Validates a personal access token via `GET /me`, then stores it.
    pub async fn sign_in_with_token(&self, token: &str) -> Result<Account> {
        let token = token.trim();
        if token.is_empty() {
            return Err(Error::Unauthorized);
        }
        let account = self.fetch_account(Auth::Token(token)).await?;
        self.tokens.save(token)?;
        Ok(account)
    }

    /// Forgets the token locally. `/oauth/logout` is not called: it would revoke
    /// the user's personal token on mod.io itself.
    pub fn sign_out(&self) -> Result<()> {
        self.tokens.clear()
    }

    /// `Err(Unauthorized)` means the stored token expired; the caller clears it.
    pub async fn account(&self) -> Result<Account> {
        if !self.signed_in()? {
            return Ok(Account::signed_out());
        }
        self.fetch_account(Auth::Bearer).await
    }

    async fn fetch_account(&self, auth: Auth<'_>) -> Result<Account> {
        let me: ApiUser = self.get_json(AUTH_GAME, "me", auth, &[]).await?;
        Ok(Account {
            signed_in: true,
            username: Some(me.username),
            expires_at: None,
            // A personal token is only issued after accepting the terms on mod.io.
            terms_accepted: true,
        })
    }
}
