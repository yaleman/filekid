//! OIDC-related views

use super::prelude::*;
use axum::http::Uri;
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
pub async fn logout(session: Session) -> Result<Redirect, (StatusCode, &'static str)> {
    session.clear().await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(Redirect::to(Urls::Index.as_ref()))
}
