//! CLI Options

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Default)]
pub struct CliOpts {
    #[clap(short, long)]
    pub config: Option<PathBuf>,

    #[clap(short, long)]
    pub debug: bool,

    #[clap(long)]
    #[cfg(any(debug_assertions, test))]
    pub oauth2_disable: bool,

    #[clap(long)]
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
