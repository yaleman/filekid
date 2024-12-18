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
}
