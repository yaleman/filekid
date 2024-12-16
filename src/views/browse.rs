//! This module contains the browse endpoint, which allows users to browse the files on the server.
use std::fs::DirEntry;
use std::path::PathBuf;

use axum::body::Bytes;
use axum::extract::{Multipart, Path};
use axum::http::HeaderMap;
use axum::response::Redirect;
use tracing::{debug, warn};

use crate::fs::{fs_from_serverpath, FileData};

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
        error!("Couldn't find file!");
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
                "Failed to parse mime type for file {:?} -> {}: {}",
                server_path_object.path, filepath, err
            );
            Error::InternalServerError(format!(
                "Failed to parse mime type for file {:?} -> {}: {}",
                server_path_object.path, filepath, err
            ))
        })?,
    );
    Ok((
        StatusCode::OK,
        headers,
        filekidfs.get_file(metadata).await??,
    ))
}

#[derive(Template)]
#[template(path = "browse.html")]
pub(crate) struct BrowsePage {
    server_path: String,
    entries: Vec<FileEntry>,
    parent_path: String,
    current_path: String,
}

#[derive(Debug)]
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

#[derive(Debug)]
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

impl TryFrom<DirEntry> for FileEntry {
    type Error = Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let path = value.path();
        let filename = path
            .file_name()
            .ok_or_else(|| Error::Generic("Couldn't get filename".to_string()))?;
        let filename = filename.to_string_lossy().to_string();
        let filetype = FileType::try_from(&path)?;
        Ok(Self {
            filename,
            fullpath: path.to_string_lossy().to_string(),
            filetype,
        })
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

    let target_filepath = filepath
        .clone()
        .map(|p| p.trim_start_matches('/').to_string())
        .clone()
        .unwrap_or("".into());

    if !filekidfs.exists(&target_filepath)? {
        error!("Couldn't find file path {:?}", target_filepath);
        return Err(Error::NotFound(filepath.unwrap_or("".into())));
    }

    let parent_path = match &filepath {
        Some(p) => {
            let mut p: Vec<_> = p.split("/").collect();
            p.pop();
            p.join("/")
        }
        None => "".to_string(),
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

pub(crate) async fn upload_nopath(
    State(state): State<WebState>,
    Path(server_path): Path<String>,
    multipart: Multipart,
) -> Result<Redirect, Error> {
    upload_file(State(state), Path((server_path, None)), multipart).await
}

pub(crate) async fn upload_file(
    State(state): State<WebState>,
    Path((server_path, filepath)): Path<(String, Option<String>)>,
    mut multipart: Multipart,
) -> Result<Redirect, Error> {
    let server_reader = state.configuration.read().await;

    let server_path_object = match server_reader.server_paths.get(&server_path) {
        None => {
            error!("Couldn't find server path {}", server_path);
            return Err(Error::NotFound(server_path));
        }
        Some(p) => p,
    };

    let filekidfs = fs_from_serverpath(server_path_object)?;

    let mut uploaded_file: Option<FileData> = None;
    let mut uploaded_data: Option<Bytes> = None;
    // let mut overwrite: bool = false;

    const FIELD_NAMES: [&str; 2] = ["file", "overwrite"];

    let stripped_filepath = filepath.clone().unwrap_or_default();

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(field_name) = field.name() {
            if !FIELD_NAMES.contains(&field_name) {
                warn!(
                    "File upload attempted using erroneous field name {} - ignoring",
                    field_name
                );
                continue;
            }

            if field_name == "file" {
                let file_name = match field.file_name() {
                    Some(name) => name.to_owned(),
                    None => {
                        warn!("File upload attempted without a filename - ignoring");
                        continue;
                    }
                };

                let full_path = [stripped_filepath.clone(), file_name.clone()].join("/");

                if filekidfs.exists(&full_path)? {
                    warn!("File {} already exists - ignoring", file_name);
                    continue;
                }

                let data = field.bytes().await.map_err(|err| {
                    error!("Failed to read file data: {:?}", err);
                    Error::InternalServerError("Failed to read file data".to_string())
                })?;

                debug!(
                    "Length of `{}`) is {} bytes",
                    file_name,
                    // content_type,
                    data.len()
                );

                uploaded_file = Some(FileData {
                    filepath: stripped_filepath.clone().into(),
                    filename: file_name,
                    size: Some(data.len() as u64),
                });
                uploaded_data = Some(data);
            } else if field_name == "overwrite" {
                // overwrite = true;
                // TODO: handle the ovewrite field
            }
        }
    }

    // have we got a file?
    match (uploaded_file, uploaded_data) {
        (Some(uploaded_file), Some(uploaded_data)) => {
            filekidfs.put_file(&uploaded_file, &uploaded_data).await?;
            Ok(Redirect::to(&format!(
                "/browse/{}/{}",
                server_path,
                filepath.unwrap_or("".to_string())
            )))
        }
        _ => {
            warn!("No file uploaded");
            Err(Error::BadRequest("No file uploaded".to_string()))
        }
    }
}
