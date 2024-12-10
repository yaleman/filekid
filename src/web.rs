//! Web UI things

use axum::routing::get;
use axum_server::bind_rustls;
use axum_server::tls_rustls::RustlsConfig;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLockReadGuard;
use tower_http::services::ServeDir;

use askama_axum::IntoResponse;
use axum::error_handling::HandleErrorLayer;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::response::Redirect;
use axum::Router;
use axum_oidc::error::MiddlewareError;
use axum_oidc::{EmptyAdditionalClaims, OidcAuthLayer, OidcLoginLayer};
use tower::ServiceBuilder;

use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};
use tracing::{debug, error, info};

use crate::constants::WEB_SERVER_DEFAULT_STATIC_PATH;
use crate::oidc::OidcErrorHandler;
use crate::views::browse::{browse, browse_nopath, get_file};
use crate::{views, Config, Error, SendableConfig, WebServerControl, WebState};

pub(crate) async fn handler_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

pub(crate) enum Urls {
    Login,
    Logout,
    Index,
    RpLogout,
    HealthCheck,
    Static,
}

impl Urls {
    pub fn as_ref(&self) -> &'static str {
        match self {
            Urls::Index => "/",
            Urls::Login => "/login",
            Urls::Logout => "/logout",
            Urls::RpLogout => "/rp_logout",
            Urls::HealthCheck => "/healthy",
            Urls::Static => "/static",
        }
    }
}

async fn up(State(_state): State<WebState>) -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// /// Create the database-backed session store
// pub fn get_session_store(db: &Arc<DatabaseConnection>) -> entities::session::ModelStore {
//     crate::db::entities::session::ModelStore::new(db.clone())
// }

pub(crate) async fn build_app(state: WebState) -> Result<Router, Error> {
    // get all the config variables we need, quickly, so we can drop the lock

    let config_reader = state.configuration.read().await;
    let oidc_issuer = config_reader.oidc_issuer.clone();
    let oidc_client_id = config_reader.oidc_client_id.clone();
    let oidc_client_secret = config_reader.oidc_client_secret.clone();
    let frontend_url = config_reader.frontend_url.clone();
    drop(config_reader);

    // let session_store = get_session_store(&state.db);
    let session_store = MemoryStore::default();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(1800)));

    let frontend_url = Uri::from_str(&frontend_url)
        .map_err(|err| Error::Configuration(format!("Failed to parse base_url: {:?}", err)))?;
    debug!("Frontend URL: {:?}", frontend_url);
    let oidc_error_handler = OidcErrorHandler::new(Some(state.web_tx.clone()));

    let oidc_login_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            error!("Failed to handle OIDC logout: {:?}", e);
            e.into_response()
        }))
        .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

    let oidc_auth_layer = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async move {
            if let MiddlewareError::SessionNotFound = e {
                error!("No OIDC session found, redirecting to logout to clear it client-side");
            } else {
                oidc_error_handler.handle_oidc_error(&e).await;
            }
            Redirect::to(Urls::Logout.as_ref()).into_response()
        }))
        .layer(
            OidcAuthLayer::<EmptyAdditionalClaims>::discover_client(
                frontend_url,
                oidc_issuer,
                oidc_client_id,
                oidc_client_secret,
                vec!["openid", "groups"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .await
            .map_err(|err| {
                error!("Failed to set up OIDC: {:?}", err);
                Error::from(err)
            })?,
        );

    let app = Router::new()
        .route(
            Urls::Login.as_ref(),
            get(Redirect::temporary(Urls::Index.as_ref())),
        )
        .route(Urls::RpLogout.as_ref(), get(views::oidc::rp_logout))
        .layer(oidc_login_service)
        .route("/browse/:server_path/", get(browse_nopath))
        .route("/browse/:server_path/*filepath", get(browse))
        .route("/get/:server_path/*filepath", get(get_file))
        // after here, the routers don't *require* auth
        .route(Urls::Index.as_ref(), get(views::home))
        .layer(oidc_auth_layer)
        // after here, the URLs cannot have auth
        .route(Urls::HealthCheck.as_ref(), get(up))
        .route(Urls::Logout.as_ref(), get(views::oidc::logout))
        .nest_service(
            Urls::Static.as_ref(),
            ServeDir::new(
                state
                    .configuration
                    .read()
                    .await
                    .static_path
                    .clone()
                    .unwrap_or(PathBuf::from(WEB_SERVER_DEFAULT_STATIC_PATH)),
            )
            .precompressed_br(),
        )
        .fallback(handler_404)
        // .layer(TraceLayer::new_for_http())
        .layer(session_layer);
    // here... we... go!
    Ok(app.with_state(state))
}

fn check_certs_exist(
    config_reader: &RwLockReadGuard<'_, Config>,
) -> Result<(PathBuf, PathBuf), Error> {
    let cert_file = config_reader.cert_file.clone();
    let cert_key = config_reader.cert_key.clone();
    if !cert_file.exists() {
        return Err(Error::Generic(format!(
            "TLS is enabled but cert_file {:?} does not exist",
            cert_file
        )));
    }

    if !cert_key.exists() {
        return Err(Error::Generic(format!(
            "TLS is enabled but cert_key {:?} does not exist",
            cert_key
        )));
    };
    Ok((cert_file, cert_key))
}

/// Start and run the web server
pub async fn start_web_server(configuration: SendableConfig, app: Router) -> Result<(), Error> {
    let configuration_reader = configuration.read().await;

    let listen_address = configuration_reader.listen_addr();
    let (cert_file, cert_key) = check_certs_exist(&configuration_reader)?;
    drop(configuration_reader);

    let tls_config = RustlsConfig::from_pem_file(&cert_file.as_path(), &cert_key.as_path())
        .await
        .map_err(|err| Error::Generic(format!("Failed to load TLS config: {:?}", err)))?;
    bind_rustls(
        listen_address.parse().map_err(|err| {
            Error::Generic(format!(
                "Failed to parse listen address {}: {:?}",
                listen_address, err
            ))
        })?,
        tls_config,
    )
    .serve(app.into_make_service())
    .await
    .map_err(|err| Error::Generic(format!("Web server failed: {:?}", err)))
}

/// Starts up the web server
pub async fn run_web_server(
    config_filepath: PathBuf,
    configuration: SendableConfig,
    // db: Arc<DatabaseConnection>,
    // registry: Arc<Registry>,
    web_tx: Sender<WebServerControl>,
    mut web_server_controller: Receiver<WebServerControl>,
) -> Result<(), Error> {
    let app = build_app(
        // TODO web_tx impl
        WebState::new(
            web_tx.clone(),
            // db,
            configuration.clone(),
            // Some(registry),
            config_filepath,
        ),
    )
    .await?;

    let frontend_url = configuration.read().await.frontend_url.clone();

    info!(
        "🐕 Starting web server on {} (listen address is {}) 🐕",
        &frontend_url,
        configuration.read().await.listen_addr()
    );

    loop {
        tokio::select! {
            server_result = start_web_server(configuration.clone(), app.clone()) => {
                match server_result {Ok(_) => {
                    error!("Web server exited cleanly");
                },
                Err(err) => {
                    error!("Web server failed: {:?}", err);
                    return Err(err)
                }}
            },
            server_message = web_server_controller.recv() => {
                match server_message {
                    Some(WebServerControl::Stop) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        info!("Web server stopping");
                        return Ok(());
                    },
                    Some(WebServerControl::StopAfter(millis)) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(millis)).await;
                        info!("Web server stopping");
                        return Ok(());
                    },
                    Some(WebServerControl::Reload) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        info!("Web server reloading");
                    },
                    Some(WebServerControl::ReloadAfter(millis)) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(millis)).await;
                        info!("Web server reloading");
                    },
                    None => {
                        error!("Web server controller channel closed");
                        return Ok(())
                    }
                }
            }
        }
    }
}
