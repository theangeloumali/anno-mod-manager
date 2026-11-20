//! Sign-in: a code mod.io emails (with the user's own key), or a pasted personal token.

use atelier_core::GameId;

use crate::api::{ApiAccessToken, ApiTerms, ApiUser};
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

    /// Asks mod.io to email a one-time security code.
    pub async fn request_email_code(&self, email: &str) -> Result<()> {
        let form = [("email", email.trim().to_owned())];
        self.post_form_json::<serde_json::Value>(AUTH_GAME, "oauth/emailrequest", Auth::Key, &form)
            .await
            .map(drop)
            .map_err(|error| match error {
                Error::Api {
                    error_ref: INVALID_INPUT_REF,
                    ..
                } => Error::InvalidEmail,
                other => other,
            })
    }

    /// Trades the emailed code for an access token, checks it with `GET /me`, then stores it.
    pub async fn sign_in_with_email_code(&self, code: &str) -> Result<Account> {
        let form = [
            ("security_code", code.trim().to_uppercase()),
            // The caller has the user accept mod.io's terms before asking for a code.
            ("terms_agreed", "true".to_owned()),
        ];
        let token: ApiAccessToken = self
            .post_form_json(AUTH_GAME, "oauth/emailexchange", Auth::Key, &form)
            .await
            .map_err(code_error)?;
        let account = self.fetch_account(Auth::Token(&token.access_token)).await?;
        self.tokens.save(&token.access_token)?;
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

const INVALID_INPUT_REF: u32 = 13009;

/// mod.io's refs for a security code: 11011 used, 11012 expired, 11013 another key's, 11014 wrong.
fn code_error(error: Error) -> Error {
    match error {
        Error::Api {
            error_ref: 11011 | 11012,
            ..
        } => Error::SecurityCodeExpired,
        Error::Api {
            error_ref: 11013 | 11014 | INVALID_INPUT_REF,
            ..
        } => Error::InvalidSecurityCode,
        other => other,
    }
}
