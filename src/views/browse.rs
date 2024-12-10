//! This module contains the browse endpoint, which allows users to browse the files on the server.
use axum::extract::Path;
use axum::http::HeaderMap;

use super::prelude::*;

// use crate::{prelude::*, FileKid};

pub(crate) async fn get_file(
    State(state): State<WebState>,

    Path((server_path, filepath)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let server_path_reader = state.configuration.read().await;
    let server_path_object = match server_path_reader.server_paths.get(&server_path) {
        None => {
            error!("Couldn't find server path {}", server_path);
            return Err(Error::NotFound(server_path));
        }
        Some(p) => p,
    };
    let full_path = server_path_object
        .path
        .join(&filepath)
        .canonicalize()
        .map_err(|e| {
            Error::Generic(format!(
                "Failed to canonicalize path: {} - error: {}",
                server_path_object.path.join(&filepath).display(),
                e
            ))
        })?;

    if !full_path.exists() {
        return Err(Error::NotFound(full_path.display().to_string()));
    }
    let mime_type = mime_guess::from_path(&full_path)
        .first_or_octet_stream()
        .to_string();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        mime_type.parse().map_err(|err| {
            error!(
                "Failed to parse mime type for file {}: {}",
                server_path_object.path.join(&filepath).display(),
                err
            );
            Error::InternalServerError(format!(
                "Failed to parse mime type for file {}: {}",
                server_path_object.path.join(&filepath).display(),
                err
            ))
        })?,
    );
    Ok((
        StatusCode::OK,
        headers,
        std::fs::read(full_path).map_err(|e| {
            error!(
                "Failed to read file {} from server {}: {}",
                server_path_object.path.join(&filepath).display(),
                server_path,
                e
            );
            Error::from(e)
        })?,
    ))
}

#[derive(Template)]
#[template(path = "browse.html")]
pub(crate) struct BrowsePage {
    server_path: String,
    entries: Vec<FileEntry>,
    parent_path: Option<String>,
    current_path: String,
}

pub(crate) enum FileType {
    Directory,
    File,
}

impl FileType {
    pub fn icon(&self) -> &'static str {
        match self {
            FileType::Directory => "folder.svg",
            FileType::File => "file.svg",
        }
    }
}

pub(crate) struct FileEntry {
    filename: String,
    fullpath: String,
    filetype: FileType,
}

impl FileEntry {
    pub fn url(&self, server_path: &impl ToString) -> String {
        match self.filetype {
            FileType::Directory => format!("/browse/{}/{}", server_path.to_string(), self.fullpath),

            FileType::File => format!("/get/{}/{}", server_path.to_string(), self.fullpath),
        }
    }
}

pub(crate) async fn browse_nopath(
    State(state): State<WebState>,
    Path(server_path): Path<String>,
) -> Result<BrowsePage, Error> {
    browse(State(state), Path((server_path, None))).await
}

// /// Browse the files in a server path.
pub(crate) async fn browse(
    State(state): State<WebState>,
    Path((server_path, filepath)): Path<(String, Option<String>)>,
) -> Result<BrowsePage, Error> {
    // let path = path.server_path.clone();
    let server_reader = state.configuration.read().await;
    let server_filepath = match server_reader.server_paths.get(&server_path) {
        None => {
            error!("Couldn't find server path {}", server_path);
            return Err(Error::NotFound(server_path));
        }
        Some(p) => p,
    };

    // // get the list of files in the path

    let target_path = server_filepath
        .path
        .join(filepath.clone().unwrap_or("".to_string()));

    let entries: Vec<FileEntry> = std::fs::read_dir(&target_path)
        .map_err(|e| {
            error!(
                "Failed to read dir {} from server {}: {}",
                server_path,
                target_path.display(),
                e
            );
            Error::from(e)
        })?
        .map(|entry| {
            entry
                .map_err(|e| {
                    error!(
                        "Failed to read dir {} from server {}: {}",
                        server_path,
                        target_path.display(),
                        e
                    );
                    Error::from(e)
                })
                .and_then(|entry| {
                    let filename = entry.file_name().into_string().map_err(|e| {
                        error!(
                            "Failed to get filename for {:?} from server {}: {:?}",
                            entry, server_path, e
                        );
                        Error::InternalServerError(format!("Invalid Filename {:?} {:?}", entry, e))
                    })?;
                    let fullpath = match &filepath {
                        Some(p) => format!("{}/{}", p, filename),
                        None => filename.clone(),
                    };

                    let filetype = entry.file_type().map_err(|e| {
                        error!(
                            "Failed to get filetype for {:?} from server {}: {:?}",
                            entry, server_path, e
                        );
                        Error::from(e)
                    })?;

                    Ok(FileEntry {
                        filename,
                        fullpath,
                        filetype: if filetype.is_dir() {
                            FileType::Directory
                        } else {
                            FileType::File
                        },
                    })
                })
        })
        .collect::<Result<Vec<FileEntry>, Error>>()?;

    let parent_path = match &filepath {
        Some(p) => {
            let mut p: Vec<_> = p.split("/").collect();
            p.pop();
            Some(p.join("/"))
        }
        None => None,
    };

    let res = BrowsePage {
        server_path,
        entries,
        parent_path,
        current_path: filepath.unwrap_or("".to_string()),
    };
    Ok(res)
}
