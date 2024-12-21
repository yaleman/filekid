//! Implementation details for the FileKid server.

// #![warn(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::complexity)]
#![deny(clippy::correctness)]
#![deny(clippy::expect_used)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::panic)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::unreachable)]
#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

pub mod cli;
pub mod config;
pub mod constants;
pub mod error;
pub mod fs;
pub mod log;
pub mod oidc;
pub(crate) mod prelude;
pub mod views;
pub mod web;

use config::Config;
use error::Error;
use fs::FileKidFsType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
/// A server path.
pub struct ServerPath {
    /// The path on disk, can be relative or absolute.
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(rename = "type")]
    pub type_: FileKidFsType,
}

pub enum WebMessage {
    Shutdown,
}
pub enum WebServerControl {
    Stop,
    StopAfter(u64),
    Reload,
    ReloadAfter(u64),
}

type SendableConfig = Arc<RwLock<Config>>;

/// The FileKid server internal state
#[derive(Clone)]
pub struct WebState {
    /// The configuration.
    pub configuration: SendableConfig,

    pub web_tx: tokio::sync::mpsc::Sender<WebServerControl>,

    pub config_filepath: PathBuf,
}

impl WebState {
    /// Create a new state.
    pub async fn new(
        web_tx: tokio::sync::mpsc::Sender<WebServerControl>,
        configuration: SendableConfig,
        config_filepath: PathBuf,
    ) -> Result<Self, Error> {
        Ok(Self {
            configuration,
            web_tx,
            config_filepath,
        })
    }
}

#[cfg(test)]
mod tests {
    use cli::CliOpts;

    use super::*;

    #[tokio::test]
    async fn test_webstate() {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let config = Arc::new(RwLock::new(
            Config::new(CliOpts::test_default()).expect("Failed to make a config"),
        ));
        let config_filepath = PathBuf::from("test");
        let state = WebState::new(tx, config, config_filepath)
            .await
            .expect("Failed to get state");
        assert_eq!(
            *state.configuration.read().await,
            Config::new(CliOpts::test_default()).expect("Failed to make a config")
        );
    }
}
