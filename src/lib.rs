//! Implementation details for the FileKid server.

#![warn(missing_docs)]
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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

pub(crate) mod prelude;
pub mod views;

fn bind_address_default() -> SocketAddr {
    #[allow(clippy::expect_used)]
    "127.0.0.1:6969"
        .parse()
        .expect("Failed to parse built-in default local address!")
}

#[derive(Deserialize, Debug, Clone)]
/// Configuration for the FileKid server.
pub struct Config {
    #[serde(default = "bind_address_default")]
    /// The bind address.
    pub bind_address: SocketAddr,
    /// The maximum request body size.
    pub default_request_body_max_bytes: Option<u64>,
    /// The server paths.
    pub server_paths: HashMap<String, ServerPath>,
}

impl Config {
    /// Load the configuration from a file.
    pub fn from_file(filename: &str) -> Result<Self, String> {
        let config = std::fs::read_to_string(filename).map_err(|e| e.to_string())?;
        serde_json::from_str(&config).map_err(|e| e.to_string())
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
}

#[derive(Deserialize, Debug, Clone)]
/// A server path.
pub struct ServerPath {
    /// The path on disk, can be relative or absolute.
    pub path: PathBuf,
}

/// The FileKid server internal state
pub struct FileKid {
    /// The configuration.
    pub config: Config,
}
