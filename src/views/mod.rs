//! Web views for FileKid.

pub mod browse;
pub mod delete;
pub mod oidc;
pub mod prelude;

use std::cmp::Ordering;
use std::path::PathBuf;

use prelude::*;

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct HomePage {
    server_paths: Vec<(String, ServerPath)>,
}

pub(crate) async fn home(State(state): State<WebState>) -> Result<HomePage, Error> {
    let mut server_paths = state
        .configuration
        .read()
        .await
        .server_paths
        .clone()
        .into_iter()
        .collect::<Vec<(String, ServerPath)>>();
    server_paths.sort_by(|(a, _), (b, _)| a.cmp(b));

    Ok(HomePage { server_paths })
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
