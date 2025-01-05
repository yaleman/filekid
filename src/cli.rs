//! CLI Options

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Default)]
pub struct CliOpts {
    #[clap(short, long, env = "FILEKID_CONFIG")]
    pub config: Option<PathBuf>,

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
        opts.config = Some(PathBuf::from("files/example-config.json"));
        opts
    }
}
