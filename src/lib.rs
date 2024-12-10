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

pub mod constants;
pub mod log;
pub mod oidc;
pub(crate) mod prelude;
pub mod views;
pub mod web;

use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::sync::Arc;

use askama_axum::IntoResponse;
use axum::http::StatusCode;
use serde::Deserialize;
use tokio::sync::RwLock;

fn bind_address_default() -> IpAddr {
    #[allow(clippy::expect_used)]
    "127.0.0.1"
        .parse()
        .expect("Failed to parse built-in default local address!")
}

type SendableConfig = Arc<RwLock<Config>>;

#[derive(Deserialize, Debug, Clone)]
/// Configuration for the FileKid server.
pub struct Config {
    #[serde(default = "bind_address_default")]
    /// The bind address.
    pub bind_address: IpAddr,
    /// The port to bind to, the default is 6969
    pub port: NonZeroU16,
    /// The maximum request body size.
    pub default_request_body_max_bytes: Option<u64>,
    /// The server paths.
    pub server_paths: HashMap<String, ServerPath>,

    /// The frontend domain
    pub frontend_domain: String,

    pub oidc_issuer: String,
    pub oidc_client_id: String,
    #[serde(default)]
    pub oidc_client_secret: Option<String>,

    pub static_path: Option<PathBuf>,

    /// Certificate file path
    pub cert_file: PathBuf,
    /// Certificate key file path
    pub cert_key: PathBuf,
    /// Where to find the thing
    pub frontend_url: String,
}

impl Config {
    /// Load the configuration from a file.
    pub fn from_file(filename: &str) -> Result<Self, String> {
        let config = std::fs::read_to_string(filename).map_err(|e| e.to_string())?;
        serde_json::from_str(&config).map_err(|e| format!("Failed to parse config: {:?}", e))
    }
    /// Check that the configuration is valid.
    pub fn startup_check(&self) -> Result<(), String> {
        for (server, server_config) in self.server_paths.iter() {
            if !server_config.path.exists() {
                return Err(format!(
                    "Server path {} does not exist: {}",
                    server,
                    server_config.path.display()
                ));
            }
        }
        Ok(())
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

#[derive(Deserialize, Debug, Clone)]
/// A server path.
pub struct ServerPath {
    /// The path on disk, can be relative or absolute.
    pub path: PathBuf,
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

/// The FileKid server internal state
#[derive(Clone, Debug)]
pub struct WebState {
    /// The configuration.
    pub configuration: SendableConfig,

    pub web_tx: tokio::sync::mpsc::Sender<WebServerControl>,

    pub config_filepath: PathBuf,
}

impl WebState {
    /// Create a new state.
    pub fn new(
        web_tx: tokio::sync::mpsc::Sender<WebServerControl>,
        configuration: SendableConfig,
        config_filepath: PathBuf,
    ) -> Self {
        Self {
            configuration,
            web_tx,
            config_filepath,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Generic error
    Generic(String),
    /// A configuration error.
    Configuration(String),
    /// An OIDC error.
    Oidc(String),
    /// Couldn't find that
    NotFound(String),
    /// Internal server error
    InternalServerError(String),
    /// IO things went bad
    Io(std::io::Error),
}

impl From<axum_oidc::error::Error> for Error {
    fn from(e: axum_oidc::error::Error) -> Self {
        Self::Oidc(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> askama_axum::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response()
    }
}
