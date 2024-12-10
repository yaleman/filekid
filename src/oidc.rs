//! OIDC handling for the web server.

use tracing::error;

use axum_oidc::error::MiddlewareError;
use tokio::sync::mpsc::Sender;

use crate::WebServerControl;

#[derive(Clone)]
pub(crate) struct OidcErrorHandler {
    web_tx: Option<Sender<WebServerControl>>,
}

const RELOAD_TIME: u64 = 1000;

impl OidcErrorHandler {
    pub fn new(web_tx: Option<Sender<WebServerControl>>) -> Self {
        Self { web_tx }
    }

    pub async fn handle_oidc_error(&self, error: &MiddlewareError) {
        if let Some(tx) = &self.web_tx {
            error!(
                "Reloading web server in {}ms due to OIDC error: {:?}",
                RELOAD_TIME, error
            );
            let _ = tx.send(WebServerControl::ReloadAfter(RELOAD_TIME)).await;
        }
    }
}
