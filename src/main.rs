use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use filekid::cli::CliOpts;
use filekid::error::Error;
use filekid::log::setup_logging;
use filekid::web::run_web_server;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), filekid::error::Error> {
    let cli = CliOpts::parse();

    setup_logging(cli.debug, true).map_err(|err| Error::Generic(err.to_string()))?;

    let config = filekid::Config::new(cli)?;
    config.startup_check()?;

    let (web_tx, web_rx) = tokio::sync::mpsc::channel(1);
    println!("Listening on {}", config.frontend_url.clone());
    let sendable_config = Arc::new(RwLock::new(config));

    run_web_server(
        PathBuf::from_str("filekid.json").expect("Failed to parse filekid.json"),
        sendable_config,
        web_tx,
        web_rx,
    )
    .await
}
