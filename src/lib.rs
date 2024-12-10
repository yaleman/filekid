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
pub mod constants;
pub mod error;
pub mod fs;
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

use cli::CliOpts;
use error::Error;
use fs::FileKidFsType;
use serde::Deserialize;
use tokio::sync::RwLock;

fn bind_address_default() -> IpAddr {
    #[allow(clippy::expect_used)]
    "127.0.0.1"
        .parse()
        .expect("Failed to parse built-in default local address!")
}

type SendableConfig = Arc<RwLock<Config>>;

#[derive(Deserialize)]
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

    /// Debug mode is on
    #[serde(default)]
    pub debug: bool,

    // Testing-only option to disable OAuth2
    #[serde(default)]
    oauth2_disabled: bool,
}

impl Config {
    pub fn new(cli: CliOpts) -> Result<Self, Error> {
        let config_filename = cli.config.unwrap_or(PathBuf::from("filekid.json"));

        let mut config = Self::from_file(&config_filename)?;

        if cli.debug {
            config.debug = true
        }
        #[cfg(any(debug_assertions, test))]
        if cli.oauth2_disable {
            config.oauth2_disabled = true
        }

        Ok(config)
    }

    /// Load the configuration from a file.
    pub fn from_file(filename: &PathBuf) -> Result<Self, Error> {
        let config = std::fs::read_to_string(filename)?;
        serde_json::from_str(&config).map_err(|e| {
            eprintln!("Failed to parse config: {:?}", e);
            Error::Configuration(e.to_string())
        })
    }
    /// Check that the configuration is valid.
    pub fn startup_check(&self) -> Result<(), Error> {
        for (server, server_config) in self.server_paths.iter() {
            if !server_config.path.exists() {
                return Err(Error::NotFound(format!(
                    "Server path {} does not exist: {}",
                    server,
                    server_config.path.display()
                )));
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
