use crate::data::commit::{Commit, CommitStatus, CommittedFile};
use crate::errors::BucketError;
use crate::utils::checks::{find_directory_in_parents, is_valid_bucket_info};
use crate::utils::utils::{find_bucket_repo, find_files_excluding_top_level_b, hash_file};
use blake3::Hash;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, io};
use toml::to_string;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub relative_bucket_path: PathBuf,
}

pub trait BucketTrait {
    fn default(uuid: Uuid, name: &String, path: &PathBuf) -> Self;
    fn from_meta_data(current_path: &PathBuf) -> Result<Bucket, BucketError>;
    fn write_bucket_info(&self) -> Result<(), io::Error>;
    #[cfg(test)]
    fn write_bucket_info_with_repo_path(&self, repo_path: &Path) -> Result<(), io::Error>;
    fn is_valid_bucket(dir_path: &Path) -> bool;
    fn find_bucket(dir_path: &Path) -> Option<PathBuf>;
    fn get_full_bucket_path(&self) -> Result<PathBuf, BucketError>;
    fn full_path(&self) -> Result<PathBuf, BucketError>;
    #[allow(dead_code)]
    fn list_files_with_metadata_in_bucket(&self) -> io::Result<Commit>;
}

impl BucketTrait for Bucket {
    fn default(uuid: Uuid, name: &String, path: &PathBuf) -> Bucket {
        // Ensure the path is always relative by stripping any leading slash
        let relative_path = if path.is_absolute() {
            // If given an absolute path, try to make it relative to the repo root
            // This shouldn't happen in production but might occur in tests
            path.components()
                .skip_while(|c| {
                    matches!(
                        c,
                        std::path::Component::RootDir | std::path::Component::Prefix(_)
                    )
                })
                .collect::<PathBuf>()
        } else {
            path.to_path_buf()
        };

        Bucket {
            id: uuid,
            name: name.to_string(),
            relative_bucket_path: relative_path,
        }
    }

    fn from_meta_data(current_path: &PathBuf) -> Result<Self, BucketError> {
        debug!("Current path {}", current_path.as_path().display());
        // find the top level of the bucket directory
        let bucket_path: PathBuf = match Bucket::find_bucket(current_path.as_path()) {
            Some(mut path) => {
                path.pop();
                path
            }
            None => {
                return Err(BucketError::NotAValidBucket);
            }
        };

        let bucket = read_bucket_info(&bucket_path)?;

        // check if it is a valid bucket
        if !Self::is_valid_bucket(bucket_path.as_path()) {
            return Err(BucketError::NotAValidBucket);
        }

        Ok(bucket)
    }

    fn write_bucket_info(&self) -> Result<(), io::Error> {
        // Use full_path for filesystem operations
        let full_path = self
            .full_path()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let mut file = File::create(full_path.join(".b").join("info"))?;
        let serialized = to_string(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        file.write_fmt(format_args!("{}", serialized))?;
        Ok(())
    }

    /// Test-friendly version of write_bucket_info that accepts an explicit repository path
    #[cfg(test)]
    fn write_bucket_info_with_repo_path(&self, repo_path: &Path) -> Result<(), io::Error> {
        let repo_root = if repo_path
            .file_name()
            .map(|name| name == ".buckets")
            .unwrap_or(false)
        {
            repo_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| repo_path.to_path_buf())
        } else {
            repo_path.to_path_buf()
        };

        let full_path = repo_root.join(&self.relative_bucket_path);
        std::fs::create_dir_all(full_path.join(".b"))?;
        let mut file = File::create(full_path.join(".b").join("info"))?;
        let serialized = to_string(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        file.write_fmt(format_args!("{}", serialized))?;
        Ok(())
    }

    fn is_valid_bucket(dir_path: &Path) -> bool {
        let bucket_path = find_directory_in_parents(dir_path, ".b");
        match bucket_path {
            Some(path) => is_valid_bucket_info(&path),
            None => false,
        }
    }

    fn find_bucket(dir_path: &Path) -> Option<PathBuf> {
        match find_directory_in_parents(dir_path, ".b") {
            Some(path) => Some(path),
            None => None,
        }
    }

    fn get_full_bucket_path(&self) -> Result<PathBuf, BucketError> {
        // Delegate to full_path for consistency
        self.full_path()
    }

    fn full_path(&self) -> Result<PathBuf, BucketError> {
        let current_dir = env::current_dir().map_err(BucketError::from)?;
        let repo_path = find_bucket_repo(&current_dir.as_path()).ok_or(BucketError::NotInRepo)?;
        let repo_root = repo_path.parent().ok_or(BucketError::NotInRepo)?;

        // Always treat relative_bucket_path as relative to repo root
        let full_path = repo_root.join(&self.relative_bucket_path);
        Ok(full_path)
    }

    fn list_files_with_metadata_in_bucket(&self) -> io::Result<Commit> {
        let mut files = Vec::new();
        let bucket_path = self
            .full_path()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        for entry in find_files_excluding_top_level_b(bucket_path.as_path()) {
            let relative_path = entry.as_path();
            let absolute_path = bucket_path.join(relative_path);

            if absolute_path.is_file() {
                match hash_file(&absolute_path) {
                    Ok(hash) => {
                        //println!("BLAKE3 hash: {}", hash);
                        files.push(CommittedFile {
                            id: Default::default(),
                            name: relative_path.to_string_lossy().into_owned(),
                            hash,
                            previous_hash: Hash::from_str(
                                "0000000000000000000000000000000000000000000000000000000000000000",
                            )
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                            status: CommitStatus::New,
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to hash file: {}", e);
                        return Err(e);
                    }
                }
            } else {
                debug!("Skipping non-file: {:?}", entry.as_path());
            }
        }

        Ok(Commit {
            bucket: "".to_string(),
            files,
            timestamp: chrono::Utc::now().to_rfc3339(),
            previous: None,
            next: None,
        })
    }
}

pub fn read_bucket_info(path: &PathBuf) -> Result<Bucket, std::io::Error> {
    let info_path = path.join(".b").join("info");
    let mut file = File::open(&info_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to open {} file: {}",
                &info_path.as_os_str().to_str().unwrap_or("<invalid path>"),
                e
            ),
        )
    })?;
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)?;

    let bucket = toml::from_str(&toml_string)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(bucket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs::create_dir_all;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use uuid::Uuid;

    // Helper function to set up test environment
    fn setup_test_environment() -> std::io::Result<(PathBuf, PathBuf)> {
        let temp_dir = tempdir()?.keep();
        let buckets_repo_path = temp_dir.as_path().join(".buckets");
        create_dir_all(&buckets_repo_path)?;
        let bucket_path = temp_dir.as_path().join("test_bucket");
        create_dir_all(&bucket_path)?;
        Ok((bucket_path, buckets_repo_path))
    }

    #[test]
    fn test_default() {
        let name = String::from("test_bucket");
        let path = PathBuf::from("some/path/.b");

        let bucket = Bucket::default(Uuid::new_v4(), &name, &path);

        assert_eq!(bucket.name, name);
        assert_eq!(bucket.relative_bucket_path, path);
    }

    #[test]
    fn test_relative_bucket_path_always_relative() {
        let name = String::from("test_bucket");

        // Test with absolute path - should be converted to relative
        let absolute_path = PathBuf::from("/absolute/path/to/bucket");
        let bucket1 = Bucket::default(Uuid::new_v4(), &name, &absolute_path);
        assert!(
            !bucket1.relative_bucket_path.is_absolute(),
            "relative_bucket_path should never be absolute, got: {:?}",
            bucket1.relative_bucket_path
        );
        assert_eq!(
            bucket1.relative_bucket_path,
            PathBuf::from("absolute/path/to/bucket")
        );

        // Test with relative path - should remain relative
        let relative_path = PathBuf::from("relative/path/to/bucket");
        let bucket2 = Bucket::default(Uuid::new_v4(), &name, &relative_path);
        assert!(
            !bucket2.relative_bucket_path.is_absolute(),
            "relative_bucket_path should remain relative, got: {:?}",
            bucket2.relative_bucket_path
        );
        assert_eq!(bucket2.relative_bucket_path, relative_path);

        // Test with various absolute path formats
        let paths = vec![
            PathBuf::from("/usr/local/bucket"),
            PathBuf::from("/home/user/projects/bucket"),
            PathBuf::from("/tmp/bucket"),
        ];

        for abs_path in paths {
            let bucket = Bucket::default(Uuid::new_v4(), &name, &abs_path);
            assert!(
                !bucket.relative_bucket_path.is_absolute(),
                "relative_bucket_path should never be absolute for path: {:?}, got: {:?}",
                abs_path,
                bucket.relative_bucket_path
            );
        }
    }

    #[test]
    fn test_write_and_read_bucket_info() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_name = String::from("test_bucket");
        let bucket_path = temp_dir.path().to_path_buf().join(&bucket_name);
        let bucket_meta_path = bucket_path.join(".b");
        create_dir_all(&bucket_meta_path)?;

        // Set up repository structure
        let buckets_repo_path = temp_dir.path().join(".buckets");
        create_dir_all(&buckets_repo_path)?;

        let bucket_default =
            Bucket::default(Uuid::new_v4(), &bucket_name, &PathBuf::from(&bucket_name));
        bucket_default
            .write_bucket_info_with_repo_path(&buckets_repo_path)
            .expect("Failed to write bucket info in test");

        let bucket = match Bucket::from_meta_data(&bucket_path) {
            Ok(bucket) => bucket,
            Err(e) => panic!("Error reading bucket info: {}", e),
        };

        assert_eq!(bucket_default, bucket);
        Ok(())
    }

    #[test]
    fn test_bucket_serialization_roundtrip() {
        let bucket = Bucket {
            id: Uuid::new_v4(),
            name: "test_bucket".to_string(),
            relative_bucket_path: PathBuf::from("path/to/bucket"),
        };

        let toml_string = toml::to_string(&bucket).expect("Failed to serialize bucket");
        let deserialized: Bucket =
            toml::from_str(&toml_string).expect("Failed to deserialize bucket");

        assert_eq!(bucket.id, deserialized.id);
        assert_eq!(bucket.name, deserialized.name);
        assert_eq!(
            bucket.relative_bucket_path,
            deserialized.relative_bucket_path
        );
    }

    #[test]
    fn test_bucket_from_meta_data_invalid_path() {
        let nonexistent_path = PathBuf::from("/definitely/does/not/exist");
        let result = Bucket::from_meta_data(&nonexistent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_bucket_from_meta_data_no_info_file() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_path = temp_dir.path().join("bucket_no_info");
        let bucket_meta_path = bucket_path.join(".b");
        create_dir_all(&bucket_meta_path)?;

        let result = Bucket::from_meta_data(&bucket_path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_bucket_from_meta_data_corrupted_info() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_path = temp_dir.path().join("bucket_corrupted");
        let bucket_meta_path = bucket_path.join(".b");
        create_dir_all(&bucket_meta_path)?;

        // Write invalid TOML to info file
        let info_path = bucket_meta_path.join("info");
        std::fs::write(&info_path, "invalid toml content { [ ] }")?;

        let result = Bucket::from_meta_data(&bucket_path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_bucket_write_info_permission_denied() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_path = temp_dir.path().join("bucket_readonly");
        let bucket_meta_path = bucket_path.join(".b");
        create_dir_all(&bucket_meta_path)?;

        let bucket = Bucket::default(
            Uuid::new_v4(),
            &"test".to_string(),
            &PathBuf::from("bucket_readonly"),
        );

        // Set up buckets directory structure
        let buckets_dir = temp_dir.path().join(".buckets");
        create_dir_all(&buckets_dir)?;

        // Make directory read-only on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&bucket_meta_path)?.permissions();
            perms.set_mode(0o444); // read-only
            std::fs::set_permissions(&bucket_meta_path, perms)?;

            let result = bucket.write_bucket_info_with_repo_path(&buckets_dir);

            // Restore permissions for cleanup
            let mut perms = std::fs::metadata(&bucket_meta_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&bucket_meta_path, perms)?;

            assert!(result.is_err());
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn test_bucket_get_full_bucket_path() -> std::io::Result<()> {
        let (bucket_path, _buckets_repo_path) = setup_test_environment()?;
        // Change to the temp directory so full_path() can find the .buckets repo
        env::set_current_dir(bucket_path.parent().unwrap())?;

        // Use relative path for bucket creation
        let bucket = Bucket::default(
            Uuid::new_v4(),
            &"test".to_string(),
            &PathBuf::from("test_bucket"),
        );

        match bucket.get_full_bucket_path() {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Failed to get full bucket path: {:?}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    #[test]
    #[serial]
    fn test_bucket_list_files_with_metadata_in_bucket_empty() -> std::io::Result<()> {
        let (bucket_path, _buckets_repo_path) = setup_test_environment()?;
        // Change to the temp directory so full_path() can find the .buckets repo
        env::set_current_dir(bucket_path.parent().unwrap())?;

        // Use relative path for bucket creation
        let bucket = Bucket::default(
            Uuid::new_v4(),
            &"empty".to_string(),
            &PathBuf::from("test_bucket"),
        );
        match bucket.list_files_with_metadata_in_bucket() {
            Ok(files) => {
                assert!(files.files.is_empty());
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to list files in bucket: {:?}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    #[test]
    #[serial]
    fn test_bucket_list_files_with_metadata_in_bucket_with_files() -> std::io::Result<()> {
        let (bucket_path, _buckets_repo_path) = setup_test_environment()?;
        // Change to the temp directory so full_path() can find the .buckets repo
        env::set_current_dir(bucket_path.parent().unwrap())?;

        // Create some test files
        std::fs::write(bucket_path.join("file1.txt"), "content1")?;
        std::fs::write(bucket_path.join("file2.txt"), "content2")?;

        // Use relative path for bucket creation
        let bucket = Bucket::default(
            Uuid::new_v4(),
            &"with_files".to_string(),
            &PathBuf::from("test_bucket"),
        );
        match bucket.list_files_with_metadata_in_bucket() {
            Ok(files) => {
                assert_eq!(files.files.len(), 2);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to list files in bucket: {:?}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    #[test]
    #[serial]
    fn test_bucket_list_files_with_metadata_in_bucket_with_subdirectories() -> std::io::Result<()> {
        let (bucket_path, _buckets_repo_path) = setup_test_environment()?;
        // Change to the temp directory so full_path() can find the .buckets repo
        env::set_current_dir(bucket_path.parent().unwrap())?;

        // Create test files in subdirectories
        let subdir = bucket_path.join("subdir");
        create_dir_all(&subdir)?;
        std::fs::write(subdir.join("file1.txt"), "content1")?;
        std::fs::write(subdir.join("file2.txt"), "content2")?;

        // Use relative path for bucket creation
        let bucket = Bucket::default(
            Uuid::new_v4(),
            &"with_subdirs".to_string(),
            &PathBuf::from("test_bucket"),
        );
        match bucket.list_files_with_metadata_in_bucket() {
            Ok(files) => {
                assert_eq!(files.files.len(), 2);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to list files in bucket: {:?}", e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    #[test]
    fn test_read_bucket_info_invalid_path() {
        let nonexistent_path = PathBuf::from("/definitely/does/not/exist");
        let result = read_bucket_info(&nonexistent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_bucket_info_valid_file() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_path = temp_dir.path().join("valid_bucket");
        let bucket_meta_path = bucket_path.join(".b");
        create_dir_all(&bucket_meta_path)?;

        // Create a valid bucket and write its info
        let buckets_dir = temp_dir.path().join(".buckets");
        create_dir_all(&buckets_dir)?;
        let original_bucket = Bucket::default(
            Uuid::new_v4(),
            &"valid_bucket".to_string(),
            &PathBuf::from("valid_bucket"),
        );
        // Create the bucket directory structure first
        create_dir_all(bucket_path.join(".b"))?;
        original_bucket.write_bucket_info_with_repo_path(&buckets_dir)?;

        // Read it back using the standalone function
        let read_bucket = read_bucket_info(&bucket_path)?;
        assert_eq!(original_bucket, read_bucket);
        Ok(())
    }

    #[test]
    fn test_bucket_fields() {
        let uuid = Uuid::new_v4();
        let name = "test_bucket".to_string();
        let path = PathBuf::from("test/path");

        let bucket = Bucket {
            id: uuid,
            name: name.clone(),
            relative_bucket_path: path.clone(),
        };

        assert_eq!(bucket.id, uuid);
        assert_eq!(bucket.name, name);
        assert_eq!(bucket.relative_bucket_path, path);
    }
}
