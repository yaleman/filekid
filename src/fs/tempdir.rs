//! Tempdir module, only works while the instance is up

use std::path::PathBuf;

use tracing::debug;

use crate::error::Error;
use crate::views::browse::FileEntry;

use super::FileKidFs;

#[derive(Debug)]
pub(crate) struct TempDir(tempfile::TempDir);

impl TempDir {
    pub fn new() -> Result<Self, crate::error::Error> {
        Ok(Self(tempfile::tempdir()?))
    }

    /// Ensure that the thing we're looking at is in a "safe" path
    fn is_in_basepath(&self, filename: &PathBuf) -> Result<bool, Error> {
        Ok(self
            .0
            .path()
            .join(filename)
            .canonicalize()?
            .ancestors()
            .any(|path| path == self.0.path()))
    }
}

impl FileKidFs for TempDir {
    fn name(&self) -> String {
        "tempdir".to_string()
    }

    fn available(&self) -> Result<bool, crate::error::Error> {
        Ok(self.0.path().exists())
    }

    fn exists(&self, filepath: &str) -> Result<bool, crate::error::Error> {
        debug!("{:?} exists: {}", self, filepath);
        Ok(true)
    }

    fn get_data(&self, path: &str) -> Result<super::FileData, crate::error::Error> {
        let target = self.0.path().join(path).canonicalize()?;
        if self.0.path().ancestors().any(|p| p == target) {
            if let Some(filename) = target.file_name() {
                Ok(super::FileData {
                    filename: filename.to_string_lossy().to_string(),
                    filepath: target.to_string_lossy().to_string(),
                    size: Some(target.metadata()?.len()),
                })
            } else {
                Err(crate::error::Error::Generic(
                    "Couldn't get filename".to_string(),
                ))
            }
        } else {
            Err(crate::error::Error::NotAuthorized(
                "Path is outside of parent path".to_string(),
            ))
        }
    }

    fn get_file(&self, filedata: &super::FileData) -> Result<Vec<u8>, crate::error::Error> {
        let target_file = self.0.path().join(&filedata.filepath).canonicalize()?;

        if !self.is_in_basepath(&target_file)? {
            return Err(Error::NotAuthorized(
                "Path is outside of base path".to_string(),
            ));
        }

        std::fs::read(target_file).map_err(|e| Error::Generic(e.to_string()))
    }

    fn put_file(
        &self,
        _filedata: &super::FileData,
        _contents: &[u8],
    ) -> Result<(), crate::error::Error> {
        todo!("tempfile upload file functionality")
    }

    fn delete_file(&self, _filedata: &super::FileData) -> Result<(), crate::error::Error> {
        todo!("tempdir delete file functionality")
    }

    fn list_dir(
        &self,
        path: Option<String>,
    ) -> Result<Vec<crate::views::browse::FileEntry>, Error> {
        let path_addition = path.unwrap_or_default();

        let target_path = self.0.path().join(path_addition);
        let mut res = Vec::new();

        if let Ok(readdir) = target_path.read_dir() {
            for direntry in readdir {
                let direntry = direntry.map_err(Error::from)?;
                res.push(FileEntry::try_from(direntry)?);
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {

    use crate::fs::FileKidFs;

    #[test]
    fn test_tempdir_get_outside_parent() {
        let tempdir = super::TempDir::new().expect("Failed to create tempdir");
        assert!(tempdir.get_data("/../../../test.txt").is_err());
    }
}