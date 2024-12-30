use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use futures::{Stream, TryStreamExt};
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio_util::io::StreamReader;

use crate::error::Error;
use crate::views::browse::FileEntry;
use crate::ServerPath;

pub mod local;
pub mod s3;
pub mod tempdir;

#[derive(Debug)]
pub struct FileData {
    /// the end of the path
    pub filename: String,
    /// the parent path on disk
    pub filepath: PathBuf,
    pub size: Option<u64>,
}

#[async_trait::async_trait]
pub trait FileKidFs
where
    Self: std::fmt::Debug + Send,
{
    fn name(&self) -> String;

    /// Does this support the stream_put_file method?
    fn has_stream_put_file(&self) -> bool {
        false
    }

    /// Does this filepath exist within the scope of this filesystem?
    fn exists(&self, filepath: &str) -> Result<bool, Error>;

    fn get_data(&self, path: &str) -> Result<FileData, Error>;

    async fn get_file(&self, filepath: &str) -> Result<Vec<u8>, Error>;
    async fn read_file(&self, filepath: &str) -> Result<axum::body::Body, Error>;

    async fn put_file(&self, filepath: &str, contents: &[u8]) -> Result<(), Error>;

    fn delete_file(&self, filepath: &str) -> Result<(), Error>;

    fn list_dir(&self, path: Option<String>) -> Result<Vec<FileEntry>, Error>;
    /// Checks if it's online/available - for S3 this would be checking if the bucket exists, local filesystem would be checking if the path exists
    fn available(&self) -> Result<bool, Error>;

    fn target_path(&self, filepath: &str, filename: &str) -> Result<String, Error> {
        if filename.is_empty() {
            return Err(Error::BadRequest("Filename is empty".to_string()));
        }
        if filepath.trim().strip_prefix('/').unwrap_or("").is_empty() {
            Ok(filename.to_string())
        } else {
            Ok(format!("{}/{}", filepath, filename))
        }
    }
    fn target_path_from_key(&self, key: &str) -> PathBuf;

    fn is_file(&self, key: &str) -> bool;
    fn is_dir(&self, key: &str) -> bool;
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileKidFsType {
    Local,
    TempDir,
}

pub fn fs_from_serverpath(server_path: &ServerPath) -> Result<Box<dyn FileKidFs>, Error> {
    match &server_path.type_ {
        FileKidFsType::Local => {
            let server_path = match server_path.path {
                Some(ref path) => path,
                None => return Err(Error::Configuration("No path specified".to_string())),
            };
            Ok(Box::new(local::LocalFs::new(server_path.to_path_buf())))
        }
        FileKidFsType::TempDir => match &server_path.path {
            None => Err(Error::Configuration(
                "No path specified for tempdir after startup?".to_string(),
            )),
            Some(path) => Ok(Box::new(tempdir::TempDir::new(path.to_owned()))),
        },
    }
}

// This code is from https://github.com/tokio-rs/axum/blob/f8f3a030b32d9a0fa52be6834fb142ea1c14f2d2/examples/stream-to-file/src/main.rs to stream to disk
// Save a `Stream` to a file
pub async fn stream_to_file<S, E>(filepath: &str, stream: S) -> Result<(), Error>
where
    S: Stream<Item = Result<axum::body::Bytes, E>>,
    E: Into<axum::BoxError>,
{
    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error =
            stream.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Create the file. `File` implements `AsyncWrite`.
        // let path = std::path::Path::new(UPLOADS_DIRECTORY).join(path);
        let mut file = BufWriter::new(File::create(filepath).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, std::io::Error>(())
    }
    .await
    .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_from_serverpath_local() {
        let server_path = ServerPath {
            type_: FileKidFsType::Local,
            path: Some(PathBuf::from("/some/local/path")),
        };
        let fs = fs_from_serverpath(&server_path);
        assert!(fs.is_ok());
        assert_eq!(fs.unwrap().name(), "local:/some/local/path");
    }

    #[test]
    fn test_fs_from_serverpath_tempdir() {
        let server_path = ServerPath {
            type_: FileKidFsType::TempDir,
            path: Some(PathBuf::from("/some/tempdir/path")),
        };
        let fs = fs_from_serverpath(&server_path);
        assert!(fs.is_ok());
        assert_eq!(fs.unwrap().name(), "tempdir (/some/tempdir/path)");
    }

    #[test]
    fn test_fs_from_serverpath_local_no_path() {
        let server_path = ServerPath {
            type_: FileKidFsType::Local,
            path: None,
        };
        let fs = fs_from_serverpath(&server_path);
        assert!(fs.is_err());
    }

    #[test]
    fn test_fs_from_serverpath_tempdir_no_path() {
        let server_path = ServerPath {
            type_: FileKidFsType::TempDir,
            path: None,
        };
        let fs = fs_from_serverpath(&server_path);
        assert!(fs.is_err());
    }
}
