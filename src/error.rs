//! Error things

use std::fmt::Display;

use askama_axum::IntoResponse;
use axum::http::StatusCode;

#[derive(Debug)]
pub enum Error {
    /// Generic error
    Generic(String),
    /// A configuration error.
    Configuration(String),
    /// An OIDC error.
    Oidc(String),
    /// Couldn't find that
    NotFound(String),
    /// Internal server error
    InternalServerError(String),
    /// IO things went bad
    Io(std::io::Error),
    /// You've askked for something you're not allowed to have
    NotAuthorized(String),
    /// Can't handle this file type yet
    InvalidFileType(String),
    /// You did something weird
    BadRequest(String),
}

impl From<axum_oidc::error::Error> for Error {
    fn from(e: axum_oidc::error::Error) -> Self {
        Self::Oidc(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> askama_axum::Response {
        let statuscode = match self {
            Error::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Oidc(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotAuthorized(_) => StatusCode::FORBIDDEN,
            Error::InvalidFileType(_) => StatusCode::BAD_REQUEST,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
        };
        (statuscode, format!("{}", self)).into_response()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(e) => write!(f, "Generic error: {}", e),
            Error::Configuration(e) => write!(f, "Configuration error: {}", e),
            Error::Oidc(e) => write!(f, "OIDC error: {}", e),
            Error::NotFound(e) => write!(f, "Not found: {}", e),
            Error::InternalServerError(e) => write!(f, "Internal server error: {}", e),
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::NotAuthorized(e) => write!(f, "Not authorized: {}", e),
            Error::InvalidFileType(e) => write!(f, "Invalid file type: {}", e),
            Error::BadRequest(e) => write!(f, "Bad request: {}", e),
        }
    }
}
