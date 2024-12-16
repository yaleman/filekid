//! local filesystem backend

use std::path::PathBuf;

use tracing::{debug, error, instrument};

use crate::error::Error;
use crate::views::browse::FileType;

use super::{FileData, FileEntry, FileKidFs};

#[derive(Debug)]
pub struct LocalFs {
    pub base_path: PathBuf,
}

impl LocalFs {
    /// Ensure that the thing we're looking at is in a "safe" path
    fn is_in_basepath(&self, filename: &PathBuf) -> Result<bool, Error> {
        Ok(self
            .base_path
            .join(filename)
            .ancestors()
            .any(|path| path == self.base_path))
    }
}

#[async_trait::async_trait]
impl FileKidFs for LocalFs {
    fn name(&self) -> String {
        format!("local:{}", self.base_path.display())
    }

    fn available(&self) -> Result<bool, Error> {
        Ok(self.base_path.exists())
    }

    #[instrument(level = "debug", skip(self))]
    fn exists(&self, filepath: &str) -> Result<bool, Error> {
        let target_file = self.base_path.join(filepath);

        debug!(
            "Checking if {} exists under base path {}",
            target_file.display(),
            self.base_path.display()
        );
        if self.base_path == target_file {
            return Ok(true);
        }

        Ok(target_file.exists() && self.is_in_basepath(&PathBuf::from(filepath))?)
    }

    #[instrument(level = "debug", skip(self))]
    fn get_data(&self, path: &str) -> Result<super::FileData, Error> {
        self.is_in_basepath(&path.into())?;

        let actual_filepath = self.base_path.join(path);

        let filename = actual_filepath
            .file_name()
            .ok_or_else(|| Error::NotFound("File not found".to_string()))?;

        Ok(FileData {
            filename: filename.to_string_lossy().to_string(),
            filepath: actual_filepath
                .parent()
                .unwrap_or(&self.base_path)
                .to_path_buf(),
            size: actual_filepath.metadata().ok().map(|m| m.len()),
        })
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_file(&self, filedata: FileData) -> Result<tokio::io::Result<Vec<u8>>, Error> {
        if !self.is_in_basepath(&filedata.target_file().into())? {
            return Err(Error::NotAuthorized(
                "Path is outside of base path".to_string(),
            ));
        }

        Ok(tokio::fs::read(filedata.target_file()).await)
    }

    #[instrument(level = "debug", skip(contents, self))]
    async fn put_file(&self, filedata: &super::FileData, contents: &[u8]) -> Result<(), Error> {
        let target_file = self
            .base_path
            .join(&filedata.filepath)
            .join(&filedata.filename);

        if !self.is_in_basepath(&target_file)? {
            return Err(Error::NotAuthorized(
                "Path is outside of base path".to_string(),
            ));
        }

        debug!("Writing to file {:?}", target_file);
        tokio::fs::write(target_file, contents)
            .await
            .map_err(Error::from)
    }

    #[instrument(level = "debug", skip(self))]
    fn delete_file(&self, filedata: &super::FileData) -> Result<(), Error> {
        let target_file = self.base_path.join(&filedata.filepath);

        if !self.is_in_basepath(&target_file)? {
            return Err(Error::NotAuthorized(
                "Path is outside of base path".to_string(),
            ));
        }
        std::fs::remove_file(target_file).map_err(Error::from)
    }

    #[instrument(level = "debug", skip(self))]
    fn list_dir(&self, path: Option<String>) -> Result<Vec<FileEntry>, Error> {
        let target_path = self.base_path.join(path.clone().unwrap_or("".to_string()));
        if !self.is_in_basepath(&target_path)? {
            return Err(Error::NotAuthorized(
                "Path is outside of base path".to_string(),
            ));
        }

        std::fs::read_dir(&target_path)
            .map_err(|e| {
                error!(
                    "Failed to read dir {} from server {:?}: {}",
                    target_path.display(),
                    self,
                    e
                );
                Error::from(e)
            })?
            .map(|entry| {
                entry
                    .map_err(|e| {
                        error!(
                            "Failed to read dir {} from server {:?}: {}",
                            target_path.display(),
                            self,
                            e
                        );
                        Error::from(e)
                    })
                    .and_then(|entry| {
                        let filename = entry.file_name().into_string().map_err(|e| {
                            error!(
                                "Failed to get filename for {:?} from server {}: {:?}",
                                entry,
                                self.base_path.display(),
                                e
                            );
                            Error::InternalServerError(format!(
                                "Invalid Filename {:?} {:?}",
                                entry, e
                            ))
                        })?;
                        let fullpath = match &path {
                            Some(p) => format!("{}/{}", p, filename),
                            None => filename.clone(),
                        };

                        let filetype = entry.file_type().map_err(|e| {
                            error!(
                                "Failed to get filetype for {:?} from server {:?}: {:?}",
                                entry, self, e
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
            .collect()
    }
}
