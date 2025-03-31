//! Tempdir module, only works while the instance is up

use std::path::PathBuf;

use tracing::*;

use crate::error::Error;
use crate::views::browse::FileEntry;

use super::FileKidFs;

#[derive(Debug)]
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Ensure that the thing we're looking at is in a "safe" path
    #[instrument(level = "debug", skip(self))]
    fn is_in_basepath(&self, key: &str) -> Result<bool, Error> {
        Ok(self.target_path_from_key(key).ancestors().any(|path| {
            if path == self.0 {
                debug!(
                    "filename: {} matches parent path {} (key={})",
                    key,
                    path.display(),
                    self.target_path_from_key(key).display()
                );
                return true;
            }
            false
        }))
    }
}

#[async_trait::async_trait]
impl FileKidFs for TempDir {
    fn name(&self) -> String {
        format!("tempdir ({})", self.0.display())
    }

    fn target_path_from_key(&self, key: &str) -> PathBuf {
        self.0.join(key)
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

        Ok(self.target_path_from_key(filepath).exists() && self.is_in_basepath(filepath)?)
    }

    #[instrument(level = "debug", skip(self))]
    fn get_data(&self, path: &str) -> Result<super::FileData, crate::error::Error> {
        let target = self.target_path_from_key(path);

        debug!(
            "Checking if {} is in base path {}",
            target.display(),
            self.0.display()
        );

        self.is_in_basepath(path)?;

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

    async fn get_file(&self, filepath: &str) -> Result<Vec<u8>, Error> {
        if !self.is_in_basepath(filepath)? {
            return Err(Error::NotAuthorized(format!(
                "Path '{}' is outside of base path",
                &filepath
            )));
        }

        Ok(tokio::fs::read(&self.target_path_from_key(filepath)).await?)
    }

    #[instrument(level = "debug", skip(self))]
    async fn read_file(&self, _filepath: &str) -> Result<axum::body::Body, Error> {
        todo!("read_file hasn't beem implemented for TempDir yet");
    }

    #[instrument(level = "debug", skip(self, contents))]
    async fn put_file(&self, filepath: &str, contents: &[u8]) -> Result<(), crate::error::Error> {
        if self.is_in_basepath(filepath)? {
            let target_path = self.target_path_from_key(filepath);
            debug!("Writing to '{}'", target_path.display());
            tokio::fs::write(target_path, contents).await?;
            Ok(())
        } else {
            Err(crate::error::Error::NotAuthorized(format!(
                "Path {} is outside of parent path",
                filepath
            )))
        }
    }

    #[instrument(level = "debug", skip(self))]
    fn delete_file(&self, filepath: &str) -> Result<(), crate::error::Error> {
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

    fn is_file(&self, _key: &str) -> bool {
        todo!()
    }
    fn is_dir(&self, _key: &str) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::fs::FileKidFs;
    use crate::log::setup_logging;
    use crate::views::FileType;

    #[test]
    fn test_tempdir_get_outside_parent() {
        let tempdir = tempdir().expect("Failed to create tempdir");
        let tempdir = TempDir::new(tempdir.path().into());
        assert!(tempdir.get_data("/../../../test.txt").is_err());
    }

    #[tokio::test]
    async fn test_localfs_name() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path().to_path_buf();

        let fs = TempDir::new(temp_dir_path.clone());

        assert!(fs.name().contains(&temp_dir_path.display().to_string()));

        assert!(fs.available().expect("Isn't available!"));
    }
    #[tokio::test]
    async fn test_list_dir() {
        let _ = setup_logging(true, true);

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path().to_path_buf();

        let mut file = File::create(temp_dir.path().join("test.txt"))
            .await
            .expect("Failed to create the test temp file");
        file.write_all(b"Hello, world!")
            .await
            .expect("failed to write to file");

        let fs = TempDir::new(temp_dir_path);

        let entries = fs.list_dir(None).expect("Failed to list dir");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "test.txt");
        assert_eq!(entries[0].fullpath, "test.txt");
        assert_eq!(entries[0].filetype, FileType::File);

        let bad_test = fs.list_dir(Some("test.txt".to_string()));
        dbg!(&bad_test);
        assert!(bad_test.is_err());

        let entries = fs
            .list_dir(Some(".".to_string()))
            .expect("Failed to list dir");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "test.txt");
        assert_eq!(entries[0].fullpath, "./test.txt");
        assert_eq!(entries[0].filetype, FileType::File);
    }

    #[test]
    fn test_get_data() {
        use super::*;

        use tempfile::tempdir;

        let _ = setup_logging(true, true);

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path().to_path_buf();

        let fs = TempDir::new(temp_dir_path);

        assert!(fs.get_data("thiscannotexist.foo").is_err());
    }

    #[tokio::test]
    async fn test_get_file() {
        use super::*;
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let _ = setup_logging(true, true);

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path().to_path_buf();

        let mut file = File::create(temp_dir.path().join("test.txt"))
            .expect("Failed to create the test temp file");
        file.write_all(b"Hello, world!")
            .expect("failed to write to file");

        let fs = TempDir::new(temp_dir_path);

        let contents = fs.get_file("test.txt").await.expect("Failed to get file");
        assert_eq!(contents, b"Hello, world!");

        let result = fs.get_file("canotgtsdaftest.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_file() {
        use super::*;
        use tempfile::tempdir;

        let _ = setup_logging(true, true);

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path().to_path_buf();

        let fs = TempDir::new(temp_dir_path.clone());

        let filename = "test.txt";
        let contents = b"Hello, world!";

        let res = fs.put_file(filename, contents).await;
        assert!(res.is_ok());

        let res = fs.get_data(filename);
        assert!(res.is_ok());
        let filedata = res.expect("Failed to get file data");
        assert_eq!(filedata.size, Some(13));

        let res = fs.get_file(filename).await;
        assert!(res.is_ok());
        assert_eq!(res.expect("Failed to run get file"), contents);

        // test putting a file outside the base path
        let outside_target_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("No parent for current dir")
            .parent()
            .expect("No parent for parent dir")
            .canonicalize()
            .expect("Can't access directory above Project dir");
        dbg!(&fs);
        dbg!(&outside_target_path);

        let outside_res = fs.put_file("../../../etc/foo.txt", contents).await;

        assert!(outside_res.is_err());
    }
}
