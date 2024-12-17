//! Tempdir module, only works while the instance is up

use std::path::PathBuf;

use tracing::*;

use crate::error::Error;
use crate::views::browse::FileEntry;

use super::{FileData, FileKidFs};

#[derive(Debug)]
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Ensure that the thing we're looking at is in a "safe" path
    #[instrument(level = "debug", skip(self))]
    fn is_in_basepath(&self, filename: &PathBuf) -> Result<bool, Error> {
        Ok(self.0.join(filename).ancestors().any(|path| path == self.0))
    }
}

#[async_trait::async_trait]
impl FileKidFs for TempDir {
    fn name(&self) -> String {
        format!("tempdir ({})", self.0.display())
    }

    fn available(&self) -> Result<bool, crate::error::Error> {
        Ok(self.0.exists())
    }

    #[instrument(level = "debug", skip(self))]
    fn exists(&self, filepath: &str) -> Result<bool, crate::error::Error> {
        if filepath.is_empty() {
            // special case since it's a fresh tempdir
            return Ok(true);
        }

        let target_file = self.0.join(filepath);

        debug!(
            "Checking if {} exists under base path {}",
            target_file.display(),
            self.0.display()
        );

        Ok(target_file.exists() && self.is_in_basepath(&PathBuf::from(filepath))?)
    }

    #[instrument(level = "debug", skip(self))]
    fn get_data(&self, path: &str) -> Result<super::FileData, crate::error::Error> {
        let target = self.0.join(path);

        debug!(
            "Checking if {} is in base path {}",
            target.display(),
            self.0.display()
        );

        self.is_in_basepath(&path.into())?;

        if let Some(filename) = target.file_name() {
            Ok(super::FileData {
                filename: filename.to_string_lossy().to_string(),
                filepath: target.parent().unwrap_or(&self.0).to_path_buf(),
                size: Some(target.metadata()?.len()),
            })
        } else {
            Err(crate::error::Error::Generic(
                "Couldn't get filename".to_string(),
            ))
        }
    }

    async fn get_file(&self, filedata: FileData) -> Result<tokio::io::Result<Vec<u8>>, Error> {
        if !self.is_in_basepath(&filedata.target_file().into())? {
            return Err(Error::NotAuthorized(format!(
                "Path '{}' is outside of base path",
                &filedata.target_file()
            )));
        }

        Ok(tokio::fs::read(&filedata.target_file()).await)
    }

    #[instrument(level = "debug", skip(self, contents))]
    async fn put_file(
        &self,
        filedata: &super::FileData,
        contents: &[u8],
    ) -> Result<(), crate::error::Error> {
        let target_path = [
            filedata.filepath.clone().to_string_lossy().to_string(),
            filedata.filename.clone(),
        ]
        .join("/");
        let target_path = target_path.trim_start_matches("/");

        if self.is_in_basepath(&target_path.into())? {
            debug!("{:?}", filedata);
            let target_file = self.0.join(&filedata.filepath).join(&filedata.filename);
            debug!("Writing to '{}'", target_file.display());

            tokio::fs::write(target_file, contents).await?;
            Ok(())
        } else {
            Err(crate::error::Error::NotAuthorized(format!(
                "Path {} is outside of parent path",
                target_path
            )))
        }
    }

    #[instrument(level = "debug", skip(self))]
    fn delete_file(&self, _filedata: &super::FileData) -> Result<(), crate::error::Error> {
        todo!("tempdir delete file functionality")
    }

    #[instrument(level = "debug", skip(self))]
    fn list_dir(
        &self,
        path: Option<String>,
    ) -> Result<Vec<crate::views::browse::FileEntry>, Error> {
        let path_addition = path.unwrap_or_default();

        let target_path = self.0.join(&path_addition);
        if !target_path.is_dir() {
            return Err(Error::BadRequest(format!(
                "{} is not a directory",
                path_addition
            )));
        }

        debug!("listing files for {}", target_path.display());
        let mut res = Vec::new();

        if let Ok(readdir) = target_path.read_dir() {
            for direntry in readdir {
                let direntry = direntry.map_err(Error::from)?;
                let mut fileentry = FileEntry::try_from(direntry)?;
                fileentry.fullpath = format!("{}/{}", path_addition, fileentry.filename)
                    .trim_start_matches("/")
                    .to_string();
                res.push(fileentry);
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {

    use tempfile::tempdir;

    use crate::fs::FileKidFs;
    use crate::log::setup_logging;
    use crate::views::browse::FileType;

    #[test]
    fn test_tempdir_get_outside_parent() {
        let tempdir = tempdir().expect("Failed to create tempdir");
        let tempdir = super::TempDir::new(tempdir.path().into());
        assert!(tempdir.get_data("/../../../test.txt").is_err());
    }

    #[tokio::test]
    async fn test_localfs_name() {
        use super::*;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let fs = TempDir::new(temp_dir_path.clone());

        assert!(fs.name().contains(&temp_dir_path.display().to_string()));
    }
    #[tokio::test]
    async fn test_list_dir() {
        use super::*;
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let _ = setup_logging(true, true);

        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        let mut file = File::create(temp_dir.path().join("test.txt")).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        let fs = TempDir::new(temp_dir_path);

        let entries = fs.list_dir(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "test.txt");
        assert_eq!(entries[0].fullpath, "test.txt");
        assert_eq!(entries[0].filetype, FileType::File);

        let bad_test = fs.list_dir(Some("test.txt".to_string()));
        dbg!(&bad_test);
        assert!(bad_test.is_err());

        let entries = fs.list_dir(Some(".".to_string())).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "test.txt");
        assert_eq!(entries[0].fullpath, "./test.txt");
        assert_eq!(entries[0].filetype, FileType::File);
    }
}
