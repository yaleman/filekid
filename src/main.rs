use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use filekid::log::setup_logging;
use filekid::web::run_web_server;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = filekid::Config::from_file("filekid.json")?;
    config.startup_check()?;

    setup_logging(true, true).map_err(|err| err.to_string())?;

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
    .map_err(|err| format!("{:?}", err))?;

    Ok(())
}
