use std::sync::Arc;
use std::time::Duration;

use atelier_core::GameId;
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use url::Url;

use crate::api::ApiPage;
use crate::error::{Error, Result};
use crate::token_store::{KeyringTokenStore, TokenStore};

/// mod.io caps `_limit` (and `id-in` lists) at 100.
pub(crate) const MAX_LIMIT: usize = 100;
const ROLLING_LIMIT_WAIT_SECS: u64 = 60;
/// Longer waits are surfaced to the UI rather than slept through.
const MAX_RETRY_WAIT_SECS: u64 = 60;
/// mod.io pages are tens of KB; this only stops a runaway body.
const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;
/// 50 pages of 100 covers any real subscription or collection list.
const MAX_PAGES: usize = 50;
const MAX_REDIRECTS: usize = 10;
pub(crate) const PLATFORM_HEADER: &str = "X-Modio-Platform";
pub(crate) const PLATFORM: &str = "windows";

pub fn modio_game_id(game: GameId) -> u64 {
    match game {
        GameId::Anno117 => 11358,
        GameId::Anno1800 => 4169,
        GameId::Humankind => 2599,
    }
}

#[derive(Clone, Default)]
pub struct ClientConfig {
    pub api_key: Option<String>,
    /// Replaces `https://g-{game}.modapi.io/v1/` (tests, proxies).
    pub base_override: Option<Url>,
}

impl std::fmt::Debug for ClientConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClientConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_override", &self.base_override)
            .finish()
    }
}

/// Which credential a request carries.
#[derive(Clone, Copy)]
pub(crate) enum Auth<'a> {
    /// The stored bearer when signed in, else the api key.
    Any,
    /// The stored bearer, or `NotSignedIn`.
    Bearer,
    /// The api key only (terms must be read before login).
    Key,
    /// A token that is not stored yet (sign-in validation).
    Token(&'a str),
}

pub struct ModioClient {
    pub(crate) http: reqwest::Client,
    api_key: Option<String>,
    pub(crate) base_override: Option<Url>,
    pub(crate) tokens: Arc<dyn TokenStore>,
}

impl ModioClient {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let base_override = config.base_override.map(with_trailing_slash);
        let http = reqwest::Client::builder()
            .user_agent(concat!("anno-mod-manager/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .redirect(redirect_policy(base_override.is_none()))
            .build()
            .map_err(Error::from_transport)?;
        Ok(Self {
            http,
            api_key: config.api_key,
            base_override,
            tokens: Arc::new(KeyringTokenStore::default()),
        })
    }

    pub fn with_token_store(mut self, store: Arc<dyn TokenStore>) -> Self {
        self.tokens = store;
        self
    }

    fn base(&self, game: GameId) -> Result<Url> {
        match &self.base_override {
            Some(base) => Ok(base.clone()),
            None => Ok(Url::parse(&format!(
                "https://g-{}.modapi.io/v1/",
                modio_game_id(game)
            ))?),
        }
    }

    pub(crate) fn request(
        &self,
        game: GameId,
        method: Method,
        path: &str,
        auth: Auth<'_>,
    ) -> Result<RequestBuilder> {
        let url = self.base(game)?.join(path)?;
        let builder = self.http.request(method, url);
        self.authorize(builder, auth)
    }

    fn authorize(&self, builder: RequestBuilder, auth: Auth<'_>) -> Result<RequestBuilder> {
        let builder = builder.header(PLATFORM_HEADER, PLATFORM);
        let token = match auth {
            Auth::Token(token) => Some(token.to_owned()),
            Auth::Any | Auth::Bearer => self.tokens.load()?,
            Auth::Key => None,
        };
        // The bearer wins over the api key whenever both exist.
        match (token, auth, &self.api_key) {
            (Some(token), _, _) => Ok(builder.bearer_auth(token)),
            (None, Auth::Bearer, _) => Err(Error::NotSignedIn),
            (None, _, Some(key)) => Ok(builder.query(&[("api_key", key)])),
            (None, _, None) => Err(Error::MissingCredentials),
        }
    }

    pub(crate) fn signed_in(&self) -> Result<bool> {
        Ok(self.tokens.load()?.is_some())
    }

    /// Sends once; on 429 waits `retry-after` and retries once.
    pub(crate) async fn execute(&self, builder: RequestBuilder) -> Result<Response> {
        let retry = builder.try_clone();
        let response = builder.send().await.map_err(Error::from_transport)?;
        if response.status() != StatusCode::TOO_MANY_REQUESTS {
            return ensure_success(response).await;
        }
        let wait = retry_after_secs(&response);
        let Some(retry) = retry.filter(|_| wait <= MAX_RETRY_WAIT_SECS) else {
            return Err(Error::RateLimited { retry_after: wait });
        };
        tokio::time::sleep(Duration::from_secs(wait)).await;
        let response = retry.send().await.map_err(Error::from_transport)?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            return Err(Error::RateLimited {
                retry_after: retry_after_secs(&response),
            });
        }
        ensure_success(response).await
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(
        &self,
        game: GameId,
        path: &str,
        auth: Auth<'_>,
        query: &[(&str, String)],
    ) -> Result<T> {
        let builder = self.request(game, Method::GET, path, auth)?.query(query);
        let body = read_capped(self.execute(builder).await?, MAX_JSON_BYTES).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// Walks every page of a list endpoint, up to the page cap.
    pub(crate) async fn get_all<T: DeserializeOwned>(
        &self,
        game: GameId,
        path: &str,
        auth: Auth<'_>,
        query: &[(&str, String)],
    ) -> Result<Vec<T>> {
        Ok(self.get_pages(game, path, auth, query).await?.0)
    }

    /// Every page, or an error when the cap cut the list short: for lists whose missing
    /// entries would be acted on, like removing unsubscribed mods.
    pub(crate) async fn get_all_complete<T: DeserializeOwned>(
        &self,
        game: GameId,
        path: &str,
        auth: Auth<'_>,
        query: &[(&str, String)],
    ) -> Result<Vec<T>> {
        match self.get_pages(game, path, auth, query).await? {
            (items, true) => Ok(items),
            (_, false) => Err(Error::IncompleteList),
        }
    }

    /// The items and whether they are the whole list.
    async fn get_pages<T: DeserializeOwned>(
        &self,
        game: GameId,
        path: &str,
        auth: Auth<'_>,
        query: &[(&str, String)],
    ) -> Result<(Vec<T>, bool)> {
        let mut items = Vec::new();
        // A total that never stops growing would otherwise page forever.
        for _ in 0..MAX_PAGES {
            let mut page_query = query.to_vec();
            page_query.push(("_limit", MAX_LIMIT.to_string()));
            page_query.push(("_offset", items.len().to_string()));
            let page: ApiPage<T> = self.get_json(game, path, auth, &page_query).await?;
            let fetched = page.result_count;
            let total = page.result_total;
            items.extend(page.data);
            if fetched == 0 || items.len() as u64 >= total {
                return Ok((items, true));
            }
        }
        Ok((items, false))
    }

    /// A form POST whose JSON answer is needed (the sign-in exchanges).
    pub(crate) async fn post_form_json<T: DeserializeOwned>(
        &self,
        game: GameId,
        path: &str,
        auth: Auth<'_>,
        form: &[(&str, String)],
    ) -> Result<T> {
        let builder = self.request(game, Method::POST, path, auth)?.form(form);
        let body = read_capped(self.execute(builder).await?, MAX_JSON_BYTES).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// A bearer-authenticated write whose response body is not needed.
    pub(crate) async fn write(
        &self,
        game: GameId,
        method: Method,
        path: &str,
        auth: Auth<'_>,
        form: &[(&str, String)],
    ) -> Result<()> {
        // Always a form, even an empty one: mod.io answers 415 to a write without its
        // Content-Type, a bodiless DELETE included.
        let builder = self.request(game, method, path, auth)?.form(form);
        self.execute(builder).await.map(drop)
    }
}

fn with_trailing_slash(mut url: Url) -> Url {
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    url
}

fn retry_after_secs(response: &Response) -> u64 {
    let seconds = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0);
    // 0 (or absent) means a rolling limit; mod.io asks for a 60s pause.
    if seconds == 0 {
        ROLLING_LIMIT_WAIT_SECS
    } else {
        seconds
    }
}

/// mod.io's download links redirect to a CDN; in production a hop off https is refused.
fn redirect_policy(https_only: bool) -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if https_only && attempt.url().scheme() != "https" {
            attempt.error("redirect away from https")
        } else {
            attempt.follow()
        }
    })
}

pub(crate) async fn read_capped(response: Response, limit: usize) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(Error::ResponseTooLarge);
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = chunk.map_err(Error::from_transport)?;
        if body.len() + chunk.len() > limit {
            return Err(Error::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn ensure_success(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = read_capped(response, MAX_JSON_BYTES).await?;
    Err(Error::from_status(status.as_u16(), &body))
}
