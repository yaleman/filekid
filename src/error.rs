//! Error things

use askama_axum::IntoResponse;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    Io(String),
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
        Self::Io(e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let e = Error::Generic("test".to_string());
        assert_eq!(format!("{}", e), "Generic error: test");

        // go through the enum variants, make each one and test the various outputs
        let e = Error::Generic("test".to_string());
        assert_eq!(format!("{}", e), "Generic error: test");
        assert_eq!(
            e.clone().into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let e = Error::Configuration("config error".to_string());
        assert_eq!(format!("{}", e), "Configuration error: config error");
        assert_eq!(
            e.clone().into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let e = Error::Oidc("oidc error".to_string());
        assert_eq!(format!("{}", e), "OIDC error: oidc error");
        assert_eq!(
            e.clone().into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let e = Error::NotFound("not found".to_string());
        assert_eq!(format!("{}", e), "Not found: not found");
        assert_eq!(e.clone().into_response().status(), StatusCode::NOT_FOUND);

        let e = Error::InternalServerError("internal error".to_string());
        assert_eq!(format!("{}", e), "Internal server error: internal error");
        assert_eq!(
            e.clone().into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let e = Error::Io("io error".to_string());
        assert_eq!(format!("{}", e), "IO error: io error");
        assert_eq!(
            e.clone().into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let e = Error::NotAuthorized("not authorized".to_string());
        assert_eq!(format!("{}", e), "Not authorized: not authorized");
        assert_eq!(e.clone().into_response().status(), StatusCode::FORBIDDEN);

        let e = Error::InvalidFileType("invalid file type".to_string());
        assert_eq!(format!("{}", e), "Invalid file type: invalid file type");
        assert_eq!(e.clone().into_response().status(), StatusCode::BAD_REQUEST);

        let e = Error::BadRequest("bad request".to_string());
        assert_eq!(format!("{}", e), "Bad request: bad request");
        assert_eq!(e.clone().into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_from_axum_oidc() {
        let e = axum_oidc::error::Error::UrlParsing(
            openidconnect::url::ParseError::RelativeUrlWithoutBase,
        );
        let e = Error::from(e);
        assert_eq!(
            format!("{}", e),
            "OIDC error: url parsing: RelativeUrlWithoutBase"
        );
    }
}
