use dropshot::ApiDescription;
use dropshot::ConfigDropshot;
use dropshot::ConfigLogging;
use dropshot::ConfigLoggingLevel;
use dropshot::ServerBuilder;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Set up a logger.
    let log = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Info,
    }
    .to_logger("minimal-example")
    .map_err(|e| e.to_string())?;

    let config = filekid::Config::from_file("filekid.json")?;
    config.startup_check()?;

    let filekid = filekid::FileKid {
        config: config.clone(),
    };

    // Describe the API.
    let mut api = ApiDescription::new();
    // Register API functions -- see detailed example or ApiDescription docs.
    api.register(filekid::views::home)
        .map_err(|e| e.to_string())?;
    api.register(filekid::views::browse::get_file)
        .map_err(|e| e.to_string())?;
    // api.register(filekid::views::browse::browse)
    //     .map_err(|e| e.to_string())?;

    // Start the server.
    let server = ServerBuilder::new(api, filekid, log)
        .config(ConfigDropshot {
            bind_address: config.bind_address,
            default_request_body_max_bytes: 10240,
            default_handler_task_mode: dropshot::HandlerTaskMode::CancelOnDisconnect,
            log_headers: Vec::new(),
        })
        .start()
        .map_err(|error| format!("failed to start server: {}", error))?;
    println!("Listening on http://{}", config.bind_address);
    server.await
}
