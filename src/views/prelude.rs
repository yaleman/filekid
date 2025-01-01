//! Load all the usual thingsuse askama::Template;
pub(crate) use axum::extract::State;

pub(crate) use crate::web::Urls;
pub(crate) use crate::ServerPath;
pub(crate) use crate::{Error, WebState};

pub(crate) use askama::Template;

pub(crate) use askama_axum::IntoResponse;
pub(crate) use axum::http::StatusCode;
pub(crate) use serde::Deserialize;
pub(crate) use tower_sessions::Session;
pub(crate) use tracing::{debug, error, instrument};

pub(crate) use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
