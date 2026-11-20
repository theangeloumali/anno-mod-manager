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
        let mut account = self.fetch_account(Auth::Token(&token.access_token)).await?;
        self.tokens.save(&token.access_token)?;
        account.expires_at = (token.date_expires > 0).then(|| iso_utc(token.date_expires));
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

/// Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`, by Howard Hinnant's civil-from-days.
fn iso_utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    let era_days = days + 719_468;
    let era = era_days.div_euclid(146_097);
    let day_of_era = era_days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}
