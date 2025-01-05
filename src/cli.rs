//! CLI Options

use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

static DEFAULT_BIND_ADDRESS: &str = "::1";
static DEFAULT_CONFIG_PATH: &str = "filekid.json";

#[derive(Parser, Debug)]
pub struct CliOpts {
    #[clap(short, long, env = "FILEKID_CONFIG", default_value = DEFAULT_CONFIG_PATH)]
    pub config: PathBuf,

    #[clap(long, env = "FILEKID_BIND_ADDRESS", default_value = DEFAULT_BIND_ADDRESS)]
    pub bind_address: IpAddr,

    #[clap(short, long, env = "FILEKID_DEBUG")]
    pub debug: bool,

    #[clap(long, env = "FILEKID_OAUTH2_DISABLE")]
    #[cfg(any(debug_assertions, test))]
    pub oauth2_disable: bool,

    #[clap(long, env = "FILEKID_DB_DEBUG")]
    pub db_debug: bool,
}

impl CliOpts {
    #[cfg(test)]
    pub fn test_default() -> Self {
        let mut opts = Self::default();
        opts.config = PathBuf::from("files/example-config.json");
        opts
    }
}

impl Default for CliOpts {
    fn default() -> Self {
        #[allow(clippy::expect_used)]
        Self {
            config: PathBuf::from(DEFAULT_CONFIG_PATH),
            debug: false,
            #[cfg(any(debug_assertions, test))]
            oauth2_disable: false,
            db_debug: false,
            bind_address: DEFAULT_BIND_ADDRESS
                .parse()
                .expect("Failed to parse ::1 as an IP address!"),
        }
    }
}

impl CliOpts {}
