//! Config and parsing things

use crate::cli::CliOpts;
use crate::error::Error;
use crate::fs::{self, FileKidFs};
use crate::ServerPath;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU16;
use std::path::PathBuf;

fn bind_address_default() -> IpAddr {
    #[allow(clippy::expect_used)]
    "127.0.0.1"
        .parse()
        .expect("Failed to parse built-in default local address!")
}

/// Defaults to 1GB (1024MB)
fn default_max_upload_mb() -> usize {
    1024
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
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

    /// Testing-only option to disable OAuth2
    #[serde(default)]
    pub(crate) oauth2_disabled: bool,

    /// Maximum upload size,  Defaults to 1024MB
    #[serde(default = "default_max_upload_mb")]
    pub max_upload_mb: usize,
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
        if !filename.exists() {
            return Err(Error::Configuration(format!(
                "Config file {} does not exist",
                filename.display()
            )));
        }

        let config = std::fs::read_to_string(filename)?;
        serde_json::from_str(&config).map_err(|e| {
            eprintln!("Failed to parse config: {:?}", e);
            Error::Configuration(e.to_string())
        })
    }
    /// Check that the configuration is valid.
    pub fn startup_check(&self) -> Result<(), Error> {
        for (server, server_config) in self.server_paths.iter() {
            match server_config.type_ {
                fs::FileKidFsType::TempDir => {
                    // it's fine!
                }
                fs::FileKidFsType::Local => {
                    let filekid: Box<dyn FileKidFs> = fs::fs_from_serverpath(server_config)?;
                    if !filekid.available()? {
                        return Err(Error::NotFound(format!(
                            "Server path {} ({:?}) is not online",
                            server, filekid,
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_config() {
        let config = Config {
            bind_address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: NonZeroU16::new(6969).unwrap(),
            default_request_body_max_bytes: None,
            server_paths: HashMap::new(),
            frontend_domain: "example.com".to_string(),
            oidc_issuer: "https://example.com".to_string(),
            oidc_client_id: "client_id".to_string(),
            oidc_client_secret: None,
            static_path: None,
            cert_file: PathBuf::from("cert.pem"),
            cert_key: PathBuf::from("key.pem"),
            frontend_url: "https://example.com".to_string(),
            debug: false,
            oauth2_disabled: false,
            max_upload_mb: 1024,
        };

        let config_str = serde_json::to_string(&config).unwrap();
        let config2: Config = serde_json::from_str(&config_str).unwrap();
        assert_eq!(config, config2);
    }
}
