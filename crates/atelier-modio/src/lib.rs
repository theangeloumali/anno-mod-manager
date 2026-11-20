//! mod.io REST client: auth, browse, subscriptions, downloads with md5 checks, and update checks.

mod api;
mod auth;
mod browse;
mod client;
mod download;
mod error;
mod social;
mod token_store;
mod types;
mod updates;

pub use client::{ClientConfig, ModioClient, modio_game_id};
pub use error::{Error, Result};
pub use token_store::{KeyringTokenStore, MemoryTokenStore, TokenStore};
pub use types::*;
