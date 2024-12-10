//! This module contains the browse endpoint, which allows users to browse the files on the server.
use axum::extract::Path;
use axum::http::HeaderMap;

use crate::fs::fs_from_serverpath;

use super::prelude::*;

// use crate::{prelude::*, FileKid};

pub(crate) async fn get_file(
    State(state): State<WebState>,

    Path((server_path, filepath)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let server_reader = state.configuration.read().await;
    let server_path_object = match server_reader.server_paths.get(&server_path) {
        None => {
            error!("Couldn't find server path {}", server_path);
            return Err(Error::NotFound(server_path));
        }
        Some(p) => p,
    };

    let filekidfs = fs_from_serverpath(server_path_object)?;

    if !filekidfs.exists(&filepath)? {
        return Err(Error::NotFound(filepath.to_string()));
    }

    let metadata = filekidfs.get_data(&filepath)?;

    let mime_type = mime_guess::from_path(&filepath)
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
    Ok((StatusCode::OK, headers, filekidfs.get_file(&metadata)?))
}

#[derive(Template)]
#[template(path = "browse.html")]
pub(crate) struct BrowsePage {
    server_path: String,
    entries: Vec<FileEntry>,
    parent_path: Option<String>,
    current_path: String,
}

pub enum FileType {
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

pub struct FileEntry {
    pub filename: String,
    pub fullpath: String,
    pub filetype: FileType,
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

    let server_path_object = match server_reader.server_paths.get(&server_path) {
        None => {
            error!("Couldn't find server path {}", server_path);
            return Err(Error::NotFound(server_path));
        }
        Some(p) => p,
    };

    let filekidfs = fs_from_serverpath(server_path_object)?;

    if !filekidfs.exists(
        &filepath
            .clone()
            .map(|p| p.trim_start_matches('/').to_string())
            .clone()
            .unwrap_or("".into()),
    )? {
        return Err(Error::NotFound(filepath.unwrap_or("".into())));
    }

    let parent_path = match &filepath {
        Some(p) => {
            let mut p: Vec<_> = p.split("/").collect();
            p.pop();
            Some(p.join("/"))
        }
        None => None,
    };

    let entries: Vec<FileEntry> = filekidfs.list_dir(filepath.clone())?;
    let res = BrowsePage {
        server_path,
        entries,
        parent_path,
        current_path: filepath.unwrap_or("".to_string()),
    };
    Ok(res)
}
