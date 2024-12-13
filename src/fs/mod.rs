use serde::Deserialize;

use crate::error::Error;
use crate::views::browse::FileEntry;
use crate::ServerPath;

pub mod local;
pub mod s3;
pub mod tempdir;

pub struct FileData {
    pub filename: String,
    pub filepath: String,
    pub size: Option<u64>,
}

pub trait FileKidFs
where
    Self: std::fmt::Debug,
{
    fn name(&self) -> String;
    /// Does this filepath exist within the scope of this filesystem?
    fn exists(&self, filepath: &str) -> Result<bool, Error>;

    fn get_data(&self, path: &str) -> Result<FileData, Error>;

    fn get_file(&self, filedata: &FileData) -> Result<Vec<u8>, Error>;

    fn put_file(&self, filedata: &FileData, contents: &[u8]) -> Result<(), Error>;

    fn delete_file(&self, filedata: &FileData) -> Result<(), Error>;

    fn list_dir(&self, path: Option<String>) -> Result<Vec<FileEntry>, Error>;
    /// Checks if it's online/available - for S3 this would be checking if the bucket exists, local filesystem would be checking if the path exists
    fn available(&self) -> Result<bool, Error>;
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FileKidFsType {
    Local,
    // S3(s3::S3Fs),
    TempDir,
}

pub fn fs_from_serverpath(server_path: &ServerPath) -> Result<Box<dyn FileKidFs>, Error> {
    match server_path.type_ {
        FileKidFsType::Local => {
            let server_path = match server_path.path {
                Some(ref path) => path,
                None => return Err(Error::Configuration("No path specified".to_string())),
            };
            Ok(Box::new(local::LocalFs {
                base_path: server_path.canonicalize()?,
            }))
        }
        FileKidFsType::TempDir => Ok(Box::new(tempdir::TempDir::new()?)),
    }
}
