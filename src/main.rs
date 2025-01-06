use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use filekid::cli::CliOpts;
use filekid::error::Error;
use filekid::fs::FileKidFsType;
use filekid::log::setup_logging;
use filekid::web::run_web_server;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), filekid::error::Error> {
    let cli = CliOpts::parse();

    setup_logging(cli.debug, cli.db_debug).map_err(|err| Error::Generic(err.to_string()))?;

    let mut config = filekid::config::Config::new(&cli)?;
    config.startup_check()?;

    let (web_tx, web_rx) = tokio::sync::mpsc::channel(1);
    println!("Listening on {}", config.frontend_url.clone());

    let mut live_tempdirs: HashMap<String, tempfile::TempDir> = HashMap::new();

    for (server, server_config) in config.server_paths.iter_mut() {
        if let FileKidFsType::TempDir = server_config.type_ {
            let tempdir = tempfile::tempdir()?;
            server_config.path = Some(tempdir.path().to_path_buf());
            live_tempdirs.insert(server.clone(), tempdir);
        }
    }

    let sendable_config = Arc::new(RwLock::new(config));

    run_web_server(
        PathBuf::from_str("filekid.json").expect("Failed to parse filekid.json"),
        sendable_config,
        web_tx,
        web_rx,
    )
    .await
}
