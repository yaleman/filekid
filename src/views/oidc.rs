//! OIDC-related views

use super::prelude::*;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum_oidc::OidcRpInitiatedLogout;

#[instrument(level = "info", skip_all, fields(post_logout_redirect_uri=?logout.uri()))]
pub async fn rp_logout(
    State(state): State<WebState>,
    session: Session,
    logout: OidcRpInitiatedLogout,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    session.clear().await;

    let url: Uri = state
        .configuration
        .read()
        .await
        .frontend_url
        .clone()
        .parse()
        .map_err(|err| {
            error!("Failed to parse redirect URL: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse redirect URL, your session has been cleared on our end.",
            )
        })?;
    Ok(logout.with_post_logout_redirect(url))
}

/// Logs the user out
pub(crate) async fn logout(session: Session) -> Result<Redirect, (StatusCode, &'static str)> {
    session.clear().await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(Redirect::to(Urls::Index.as_ref()))
}

#[cfg(test)]
pub(crate) const OIDC_TEST_USERNAME: &str = "testuser@example.com";

#[cfg(test)]
/// Use this when you want to be "authenticated"
pub(crate) fn test_user_claims() -> OidcClaims<EmptyAdditionalClaims> {
    use std::str::FromStr;

    use openidconnect::url::Url;
    use openidconnect::{IssuerUrl, StandardClaims, SubjectIdentifier};

    OidcClaims::<EmptyAdditionalClaims>(openidconnect::IdTokenClaims::new(
        IssuerUrl::from_url(Url::from_str("https://example.com").expect("Failed to parse URL")),
        vec![],
        chrono::Utc::now() + chrono::Duration::hours(1),
        chrono::Utc::now(),
        StandardClaims::new(SubjectIdentifier::new(OIDC_TEST_USERNAME.to_string())),
        EmptyAdditionalClaims {},
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use tower_sessions::MemoryStore;
    use tower_sessions::Session;

    #[tokio::test]
    async fn test_logout() {
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        let response = logout(session).await;

        assert!(response.is_ok());
        let redirect = response.expect("Failed to get response").into_response();
        assert_eq!(redirect.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            redirect
                .headers()
                .get("location")
                .expect("Failed to get location header"),
            Urls::Index.as_ref()
        );
    }
}
