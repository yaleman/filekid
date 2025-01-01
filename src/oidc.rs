//! OIDC handling for the web server.

use axum_oidc::{AdditionalClaims, EmptyAdditionalClaims, OidcClaims};
use tracing::error;

use axum_oidc::error::MiddlewareError;
use tokio::sync::mpsc::Sender;

use crate::error::Error;
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

#[derive(Debug)]
pub(crate) struct User {
    username: String,
}

impl User {
    pub fn username(&self) -> String {
        self.username.to_owned()
    }
}

impl<AC> From<OidcClaims<AC>> for User
where
    AC: AdditionalClaims,
{
    fn from(value: OidcClaims<AC>) -> Self {
        let username = match value.preferred_username() {
            Some(username) => username.as_str().to_string(),
            None => value.subject().as_str().to_string(),
        };

        Self { username }
    }
}

pub(crate) fn check_login(
    claims: Option<OidcClaims<EmptyAdditionalClaims>>,
) -> Result<User, Error> {
    match claims {
        Some(user) => Ok(User::from(user)),
        None => Err(Error::NotAuthorized(
            "You must be logged in to view this page!".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::log::setup_logging;
    use crate::views::oidc::{test_user_claims, OIDC_TEST_USERNAME};

    use super::*;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn test_oidc_error_handler() {
        let _ = setup_logging(true, true).expect("Failed to set up logging");

        let (tx, mut rx) = channel(1);
        let handler = OidcErrorHandler::new(Some(tx));
        handler
            .handle_oidc_error(&MiddlewareError::CsrfTokenInvalid)
            .await;
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, WebServerControl::ReloadAfter(RELOAD_TIME));
    }

    #[test]
    fn test_user_from_oidc_claims() {
        let claims = test_user_claims();
        let user = User::from(claims.clone());
        assert_eq!(user.username(), OIDC_TEST_USERNAME);

        let user = check_login(Some(claims)).expect("Failed to check login");
        assert_eq!(user.username(), OIDC_TEST_USERNAME);
    }
}
