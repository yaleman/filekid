//! Web views for FileKid.

pub mod browse;
pub mod delete;
pub mod oidc;
pub mod prelude;

use std::cmp::Ordering;
use std::path::PathBuf;

use axum::response::{Html, Response};
use prelude::*;

use crate::oidc::check_login;

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct HomePage {
    server_paths: Vec<(String, ServerPath)>,
    username: String,
}

impl From<HomePage> for Result<Response, Error>
where
    HomePage: Template,
{
    fn from(page: HomePage) -> Result<Response, Error> {
        Ok(Html(page.render()?).into_response())
    }
}

pub(crate) async fn home(
    State(state): State<WebState>,
    claims: Option<OidcClaims<EmptyAdditionalClaims>>,
) -> Result<Response, Error> {
    let user = check_login(claims)?;
    debug!("User {} logged in", user.username());

    let mut server_paths = state
        .configuration
        .read()
        .await
        .server_paths
        .clone()
        .into_iter()
        .collect::<Vec<(String, ServerPath)>>();
    server_paths.sort_by(|(a, _), (b, _)| a.cmp(b));

    HomePage {
        server_paths,
        username: user.username(),
    }
    .into()
}

#[derive(Debug, Eq, PartialEq)]
pub enum FileType {
    Directory,
    File,
}

impl PartialOrd for FileType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileType {
    /// This puts the directories first in the list.
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            FileType::Directory => match other {
                FileType::Directory => Ordering::Equal,
                FileType::File => Ordering::Less,
            },
            FileType::File => match other {
                FileType::Directory => Ordering::Less,
                FileType::File => Ordering::Greater,
            },
        }
    }
}

impl FileType {
    pub fn icon(&self) -> &'static str {
        match self {
            FileType::Directory => "folder.svg",
            FileType::File => "file.svg",
        }
    }
}

impl TryFrom<&PathBuf> for FileType {
    type Error = Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        if value.is_file() {
            Ok(Self::File)
        } else if value.is_dir() {
            Ok(Self::Directory)
        } else {
            Err(Error::InvalidFileType(value.display().to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use oidc::test_user_claims;

    use super::*;

    #[tokio::test]
    async fn test_home() {
        let _ = home(
            WebState::test_webstate().await.to_state(),
            Some(test_user_claims()),
        )
        .await
        .expect("Failed to render home page");
    }

    #[test]
    fn test_filetype() {
        let file = PathBuf::from("Cargo.toml");
        let dir = PathBuf::from("src/");

        assert_eq!(
            FileType::try_from(&file).expect("failed to convert filetype"),
            FileType::File
        );
        assert_eq!(
            FileType::try_from(&dir).expect("failed to convert filetype"),
            FileType::Directory
        );

        assert!(FileType::Directory < FileType::File);

        assert_eq!(FileType::Directory.icon(), "folder.svg");
        assert_eq!(FileType::File.icon(), "file.svg");
    }
}
