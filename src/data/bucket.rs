use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use log::debug;
use serde_derive::{Deserialize, Serialize};
use toml::to_string;
use uuid::Uuid;
use crate::utils::checks::{find_bucket, find_directory_in_parents, is_valid_bucket_info};
use crate::utils::errors::BucketError;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub relative_bucket_path: PathBuf,
}

impl Bucket {
    pub(crate) fn default(uuid: Uuid, name: &String, path: &PathBuf) -> Bucket {
        Bucket {
            id: uuid,
            name: name.to_string(),
            relative_bucket_path: path.to_path_buf(),
        }
    }

    pub(crate) fn from_meta_data(current_path: PathBuf) -> Result<Self, BucketError> {
        debug!("this is a debug {}", "message");
        // find the top level of the bucket directory
        let bucket_path: PathBuf = match find_bucket(current_path.as_path()) {
            Some(mut path) => {
                path.pop();
                path
            }
            None => {
                return Err(BucketError::NotAValidBucket);
            }
        };

        let bucket = read_bucket_info(&bucket_path.join(".b"))?;

        // check if it is a valid bucket
        if !Self::is_valid_bucket(bucket_path.as_path()) {
            return Err(BucketError::NotAValidBucket);
        }

        Ok(bucket)
    }

    pub fn write_bucket_info(&self) {
        let mut file = File::create(self.relative_bucket_path.join("info")).unwrap();
        file.write_fmt(format_args!("{}", to_string(self).unwrap()))
            .unwrap();
    }


    pub fn is_valid_bucket(dir_path: &Path) -> bool {
        let bucket_path = find_directory_in_parents(dir_path, ".b");
        match bucket_path {
            Some(path) => is_valid_bucket_info(&path),
            None => false,
        }
    }
}

fn read_bucket_info(path: &PathBuf) -> Result<Bucket, std::io::Error> {
    let mut file = File::open(path.join("info"))?;
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)?;

    let bucket = toml::from_str(&toml_string)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(bucket)
}

#[cfg(test)]
mod tests {
    use std::fs::create_dir;
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_default() {
        let name = String::from("test_bucket");
        let path = PathBuf::from("/some/path/.b");

        let bucket = Bucket::default(Uuid::new_v4(), &name, &path);

        assert_eq!(bucket.name, name);
        assert_eq!(bucket.relative_bucket_path, path);
    }

    #[test]
    fn test_write_and_read_bucket_info() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_name = String::from("test_bucket");
        let bucket_path = temp_dir.path().to_path_buf().join(".b");
        create_dir(&bucket_path)?;

        let bucket_default = Bucket::default(Uuid::new_v4(), &bucket_name, &bucket_path);
        bucket_default.write_bucket_info();

        let bucket = match Bucket::from_meta_data(bucket_path) {
            Ok(bucket) => bucket,
            Err(e) => panic!("Error reading bucket info: {}", e),
        };

        assert_eq!(bucket_default, bucket);
        Ok(())
    }
}