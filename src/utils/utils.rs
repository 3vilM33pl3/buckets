use crate::errors::BucketError;
use blake3::{Hash, Hasher};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use walkdir::{DirEntry, WalkDir};

#[allow(dead_code)]
pub fn delete_and_create_tmp_dir(bucket_path: &PathBuf) -> Result<PathBuf, BucketError> {
    let tmp_bucket_path = bucket_path.join(".b").join("tmp");
    fs::remove_dir_all(&tmp_bucket_path).unwrap_or_default();
    fs::create_dir_all(&tmp_bucket_path)?;
    Ok(tmp_bucket_path)
}

pub(crate) fn find_files_excluding_top_level_b(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| is_not_in_dir(entry, dir, ".b"))
        .filter_map(|entry| make_relative_path(entry.path(), dir))
        .collect()
}

fn is_not_in_dir(entry: &DirEntry, root_dir: &Path, excluded_dir: &str) -> bool {
    let is_top_level_ex_dir = entry.depth() == 1 && entry.file_name() == excluded_dir;

    let root_exclude = root_dir.join(excluded_dir);

    let is_inside_top_level_ex_dir = entry.path().starts_with(&root_exclude);
    let is_file_or_symlink = entry.file_type().is_file() || entry.file_type().is_symlink();
    is_file_or_symlink && !is_top_level_ex_dir && !is_inside_top_level_ex_dir
}

fn make_relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(PathBuf::from)
}

pub(crate) fn hash_file<P: AsRef<Path>>(path: P) -> io::Result<Hash> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = [0; 1024]; // Buffer for reading chunks

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break; // End of file
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize())
}

pub fn find_directory_in_parents(start_path: &Path, target_dir_name: &str) -> Option<PathBuf> {
    let mut current_path = start_path;

    let potential_target = current_path.join(target_dir_name);
    if potential_target.is_dir() && fs::metadata(&potential_target).is_ok() {
        return Some(potential_target);
    }

    while let Some(parent) = current_path.parent() {
        let potential_target = parent.join(target_dir_name);
        if potential_target.is_dir() && fs::metadata(&potential_target).is_ok() {
            return Some(potential_target);
        }
        current_path = parent;
    }

    None
}

pub fn find_bucket_path(dir_path: &Path) -> Option<PathBuf> {
    find_directory_in_parents(dir_path, ".b")
    .map(|mut path| {
        path.pop();
        path
    })
}

pub fn find_bucket_repo(dir_path: &Path) -> Option<PathBuf> {
    find_directory_in_parents(dir_path, ".buckets")
}

/// Validates that the current directory is inside a valid repository.
/// This function checks for the presence of a `.buckets` directory in the parent hierarchy.
pub fn validate_repo() -> Result<(), BucketError> {
    let current_dir = env::current_dir()?;

    let _buckets_path = match find_directory_in_parents(&current_dir, ".buckets") {
        Some(path) => path,
        None => return Err(BucketError::NotInRepo),
    };

    // For now, just check if we're in a valid repo
    // The actual database connections are handled by the PostgreSQL module
    Ok(())
}

/// Helper for compatibility - PostgreSQL connections are managed automatically
pub fn close_connection(_connection: ()) -> Result<(), BucketError> {
    // PostgreSQL connections are managed by the connection pool
    // No explicit close needed
    Ok(())
}

/// Compatibility wrapper for database operations
/// This is a temporary bridge while migrating from DuckDB to PostgreSQL
///
/// TODO(buckets-db-migration): This wrapper accepts a closure that takes `&()` (unit reference)
/// instead of an actual database connection. This is a temporary measure and does not provide
/// the intended functionality. All usages should be migrated to use the new PostgreSQL connection
/// interface by Q3 2024, after which this function should be removed.
pub fn with_db_connection<F, R>(f: F) -> Result<R, BucketError>
where
    F: FnOnce(&()) -> Result<R, BucketError>,
{
    // For now, just pass a dummy connection
    // Individual commands will need to be updated to use PostgreSQL directly
    let dummy_connection = ();
    f(&dummy_connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;
    use tempfile::tempdir;

    #[test]
    fn test_delete_and_create_tmp_dir() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let bucket_tmp_path = temp_dir.path().join("bucket").join(".b").join("tmp");
        create_dir_all(&bucket_tmp_path).expect("failed to create tmp dir");

        let bucket_path = temp_dir.path().join("bucket");
        let result = delete_and_create_tmp_dir(&bucket_path);
        assert!(result.is_ok());
        assert!(bucket_path.join(".b").join("tmp").exists());
    }

    #[test]
    fn test_delete_and_create_tmp_dir_not_exist() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let bucket_b_path = temp_dir.path().join("bucket").join(".b");
        create_dir_all(&bucket_b_path).expect("failed to create .b dir");

        let bucket_path = temp_dir.path().join("bucket");
        let result = delete_and_create_tmp_dir(&bucket_path);
        assert!(result.is_ok());
        assert!(bucket_path.join(".b").join("tmp").exists());
    }

    #[test]
    fn test_is_not_in_dir() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let dir_path = temp_dir.path();

        // Create test files and directories
        // ./file1.txt
        // ./.b/file2.txt
        // ./subdir/file3.txt
        // ./subdir/subsubdir/file4.txt
        // ./.b/subsubdir/file5.txt
        fs::create_dir_all(dir_path.join(".b").join("subsubdir"))
            .expect("failed to create .b/subsubdir");
        fs::create_dir_all(dir_path.join("subdir").join("subsubdir"))
            .expect("failed to create subdir/subsubdir");
        fs::write(dir_path.join("file1.txt"), b"file1").expect("failed to write file1.txt");
        fs::write(dir_path.join(".b").join("file2.txt"), b"file2")
            .expect("failed to write .b/file2.txt");
        fs::write(dir_path.join("subdir").join("file3.txt"), b"file3")
            .expect("failed to write subdir/file3.txt");
        fs::write(
            dir_path.join("subdir").join("subsubdir").join("file4.txt"),
            b"file4",
        )
        .expect("failed to write subdir/subsubdir/file4.txt");
        fs::write(
            dir_path.join(".b").join("subsubdir").join("file5.txt"),
            b"file5",
        )
        .expect("failed to write .b/subsubdir/file5.txt");

        let root_dir = dir_path;

        let entry_file1 = WalkDir::new(dir_path.join("file1.txt"))
            .into_iter()
            .next()
            .expect("failed to get entry")
            .expect("failed to get entry");
        let entry_file2 = WalkDir::new(dir_path.join(".b").join("file2.txt"))
            .into_iter()
            .next()
            .expect("failed to get entry")
            .expect("failed to get entry");
        let entry_file3 = WalkDir::new(dir_path.join("subdir").join("file3.txt"))
            .into_iter()
            .next()
            .expect("failed to get entry")
            .expect("failed to get entry");
        let entry_file4 = WalkDir::new(dir_path.join("subdir").join("subsubdir").join("file4.txt"))
            .into_iter()
            .next()
            .expect("failed to get entry")
            .expect("failed to get entry");
        let entry_file5 = WalkDir::new(dir_path.join(".b").join("subsubdir").join("file5.txt"))
            .into_iter()
            .next()
            .expect("failed to get entry")
            .expect("failed to get entry");

        assert!(is_not_in_dir(&entry_file1, root_dir, ".b"));
        assert!(!is_not_in_dir(&entry_file2, root_dir, ".b"));
        assert!(is_not_in_dir(&entry_file3, root_dir, ".b"));
        assert!(is_not_in_dir(&entry_file4, root_dir, ".b"));
        assert!(!is_not_in_dir(&entry_file5, root_dir, ".b"));
        assert!(is_not_in_dir(&entry_file5, root_dir, "other"));

        temp_dir.close().expect("failed to delete temp dir");
    }
    #[test]
    fn test_make_relative_path() {
        let base = Path::new("/base/dir");
        let path = Path::new("/base/dir/subdir/file.txt");
        let result = make_relative_path(path, base);
        assert_eq!(result, Some(PathBuf::from("subdir/file.txt")));

        let path_outside_base = Path::new("/other/dir/file.txt");
        let result = make_relative_path(path_outside_base, base);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_files_excluding_top_level_b() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let dir_path = temp_dir.path();

        // Create files and directories
        fs::create_dir_all(dir_path.join(".b").join("subdir")).expect("failed to create .b/subdir");
        fs::create_dir_all(dir_path.join("subdir")).expect("failed to create subdir");
        fs::write(dir_path.join("file1.txt"), b"file1").expect("failed to write file1.txt");
        fs::write(dir_path.join(".b").join("file2.txt"), b"file2")
            .expect("failed to write .b/file2.txt");
        fs::write(dir_path.join("subdir").join("file3.txt"), b"file3")
            .expect("failed to write subdir/file3.txt");

        // Collect relative paths of all files, excluding `.b` directory
        let files = find_files_excluding_top_level_b(dir_path);

        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("file1.txt")));
        assert!(files.contains(&PathBuf::from("subdir/file3.txt")));
    }

    #[test]
    fn test_hash_file() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");

        // Write some content to the file
        fs::write(&file_path, b"hello world").expect("failed to write to file");

        // Compute hash
        let hash = hash_file(&file_path).expect("failed to hash file");

        // Compute the expected hash using Blake3
        let expected_hash = blake3::hash(b"hello world");

        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn test_find_directory_in_parents() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path().join("base");
        let sub_dir = base_dir.join("subdir");

        // Create directories
        fs::create_dir_all(&sub_dir).expect("failed to create subdirectory");
        fs::create_dir_all(base_dir.join(".buckets")).expect("failed to create .buckets directory");

        // Find `.buckets` directory starting from the subdirectory
        let result = find_directory_in_parents(&sub_dir, ".buckets");
        assert_eq!(result, Some(base_dir.join(".buckets")));

        // Find `.buckets` starting from the base directory
        let result = find_directory_in_parents(&base_dir, ".buckets");
        assert_eq!(result, Some(base_dir.join(".buckets")));

        // Find `.buckets` when it doesn't exist
        let result = find_directory_in_parents(temp_dir.path(), ".buckets");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_bucket_repo() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path().join("base");
        let sub_dir = base_dir.join("subdir");

        // Create `.buckets` directory
        fs::create_dir_all(base_dir.join(".buckets")).expect("failed to create .buckets directory");

        // Find bucket repo starting from a subdirectory
        let result = find_bucket_repo(&sub_dir);
        assert_eq!(result, Some(base_dir.join(".buckets")));

        // Find bucket repo starting from the base directory
        let result = find_bucket_repo(&base_dir);
        assert_eq!(result, Some(base_dir.join(".buckets")));

        // Bucket repo doesn't exist
        let result = find_bucket_repo(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_connect_to_db() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        let child_dir = temp_dir.path().join("child");

        // Create `.buckets` directory
        fs::create_dir_all(&buckets_dir).expect("failed to create .buckets directory");
        fs::create_dir_all(&child_dir).expect("failed to create child directory");

        // Change the current directory to the child directory
        env::set_current_dir(&child_dir).expect("failed to change directory");

        // Connect to the database using the function
        let result = connect_to_db();
        assert!(result.is_ok());
    }

    #[test]
    fn test_connect_to_db_invalid_database() {
        let temp_dir = tempdir().expect("failed to create temp dir");

        env::set_current_dir(&temp_dir).expect("failed to change directory");

        // Should fail because we're not in a .buckets directory
        let result = connect_to_db();
        assert!(result.is_err());
    }

    #[test]
    fn test_with_db_connection_success() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir_all(&buckets_dir).expect("failed to create .buckets directory");

        env::set_current_dir(&temp_dir).expect("failed to change directory");

        let result = with_db_connection(|_connection| {
            // Just return a test value for now
            Ok(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_db_connection_error_propagation() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir_all(&buckets_dir).expect("failed to create .buckets directory");

        env::set_current_dir(&temp_dir).expect("failed to change directory");

        let result: Result<i32, BucketError> =
            with_db_connection(|_connection| Err(BucketError::NotInRepo));

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::NotInRepo => {}
            _ => panic!("Expected NotInRepo error"),
        }
    }

    #[test]
    fn test_hash_file_large_file() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("large_file.txt");

        // Create a 1MB file
        let large_content = vec![b'A'; 1024 * 1024];
        fs::write(&file_path, &large_content)?;

        let result = hash_file(&file_path);
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert_ne!(hash, Hash::from([0u8; 32])); // Should not be zero hash
        Ok(())
    }

    #[test]
    fn test_hash_file_permission_denied() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("restricted_file.txt");

        fs::write(&file_path, "content")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&file_path)?.permissions();
            perms.set_mode(0o000); // No permissions
            fs::set_permissions(&file_path, perms)?;

            let result = hash_file(&file_path);

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&file_path)?.permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&file_path, perms)?;

            assert!(result.is_err());
        }

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_find_files_permission_denied_directory() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let restricted_dir = temp_dir.path().join("restricted");
        fs::create_dir_all(&restricted_dir)?;
        env::set_current_dir(&restricted_dir)?;

        // Create a file in the restricted directory
        fs::write(restricted_dir.join("file.txt"), "content")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&restricted_dir)?.permissions();
            perms.set_mode(0o000); // No permissions
            fs::set_permissions(&restricted_dir, perms)?;

            let result = find_files_excluding_top_level_b(temp_dir.path());

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&restricted_dir)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&restricted_dir, perms)?;

            // Should complete without crashing, may or may not include the restricted file
            assert!(!result.is_empty());
        }

        Ok(())
    }

    #[test]
    fn test_find_files_with_nested_b_directories() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");

        // Create nested structure with a .b directory in the top level (top of the bucket, not the repo)
        let nested_dir = temp_dir.path().join(".b").join("subdir").join("storage");
        fs::create_dir_all(&nested_dir)?;
        fs::write(nested_dir.join("file.txt"), "content")?;

        // Create normal files
        fs::write(temp_dir.path().join("normal.txt"), "content")?;
        fs::create_dir_all(temp_dir.path().join("subdir2"))?;
        fs::write(
            temp_dir.path().join("subdir2").join("file2.txt"),
            "content2",
        )?;

        let files = find_files_excluding_top_level_b(temp_dir.path());

        // Should find normal files but not files in .b directories
        let file_names: Vec<String> = files
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        assert!(file_names.contains(&"normal.txt".to_string()));
        assert!(file_names.iter().any(|name| name.contains(&format!(
            "subdir2{}{}",
            std::path::MAIN_SEPARATOR,
            "file2.txt"
        ))));
        assert!(!file_names.iter().any(|name| name.contains(".b")));

        Ok(())
    }

    #[test]
    fn test_make_relative_path_edge_cases() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base = temp_dir.path();

        // Test with same path
        let result = make_relative_path(base, base);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from(""));

        // Test with parent path
        let parent = base.parent().unwrap();
        let result = make_relative_path(parent, base);
        assert!(result.is_none()); // Can't make parent relative to child

        // Test with unrelated path
        let unrelated = PathBuf::from("/completely/different/path");
        let result = make_relative_path(base, &unrelated);
        // This should work or fail gracefully depending on the implementation
        let _ = result; // Just ensure it doesn't panic

        Ok(())
    }

    #[test]
    fn test_make_relative_path_file_to_base() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base = temp_dir.path();
        let sub_file = base.join("subdir").join("file.txt");

        fs::create_dir_all(sub_file.parent().unwrap())?;
        fs::write(&sub_file, "content")?;

        let result = make_relative_path(&sub_file, base);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from("subdir").join("file.txt"));

        Ok(())
    }

    #[test]
    fn test_find_bucket_repo_nested_case() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir_all(&buckets_dir)?;

        // Create deeply nested directory structure
        let deep_nested = temp_dir.path().join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep_nested)?;

        let result = find_bucket_repo(&deep_nested);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), buckets_dir);

        Ok(())
    }

    #[test]
    fn test_find_bucket_repo_no_repo() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let no_repo_dir = temp_dir.path().join("no_repo");
        fs::create_dir_all(&no_repo_dir)?;

        let result = find_bucket_repo(&no_repo_dir);
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_get_db_path_no_repo() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        env::set_current_dir(&temp_dir).expect("failed to change directory");

        let result = get_db_path();
        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::NotInRepo => {}
            _ => panic!("Expected NotInRepo error"),
        }
    }

    #[test]
    fn test_get_db_path_success() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir_all(&buckets_dir)?;

        env::set_current_dir(&temp_dir).expect("failed to change directory");

        let result = get_db_path();
        assert!(result.is_ok());

        let db_path = result.unwrap();
        let expected_path = buckets_dir.join("buckets.db");

        // On macOS, /var is a symlink to /private/var, so we need to handle this
        // Instead of comparing paths directly, check if they resolve to the same file
        let db_path_str = db_path.to_string_lossy();
        let expected_path_str = expected_path.to_string_lossy();

        // Check if both paths end with the same relative part or are canonically equivalent
        assert!(
            db_path_str == expected_path_str
                || db_path_str.ends_with("/.buckets/buckets.db")
                    && expected_path_str.ends_with("/.buckets/buckets.db")
                || db_path.canonicalize().ok() == expected_path.canonicalize().ok(),
            "Expected db_path: {:?}, got: {:?}",
            expected_path,
            db_path
        );

        Ok(())
    }

    #[test]
    fn test_connect_to_db_with_path_invalid() {
        let invalid_path = PathBuf::from("/definitely/does/not/exist/db.db");
        let result = connect_to_db_with_path(&invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_to_db_with_path_valid() -> io::Result<()> {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        // Test the compatibility wrapper
        let result = connect_to_db_with_path(&db_path);
        assert!(result.is_ok());

        Ok(())
    }
}

/// Get the database path without opening a connection
#[allow(dead_code)]
pub fn get_db_path() -> Result<std::path::PathBuf, BucketError> {
    let current_dir = env::current_dir()?;

    match find_directory_in_parents(&current_dir, ".buckets") {
        Some(path) => Ok(path.join("buckets.db")),
        None => Err(BucketError::NotInRepo),
    }
}

/// Create a database connection from a path (useful for reusing path lookups)
/// This is a compatibility wrapper - PostgreSQL connections are managed differently
#[allow(dead_code)]
pub fn connect_to_db_with_path(db_path: &std::path::Path) -> Result<(), BucketError> {
    // Check if the path is valid - for PostgreSQL migration compatibility
    if !db_path.exists() {
        let parent = db_path.parent();
        if let Some(parent_dir) = parent {
            if !parent_dir.exists() {
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Database directory does not exist",
                )));
            }
        } else {
            return Err(BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid database path",
            )));
        }
    }

    // PostgreSQL connections are handled by the connection pool
    Ok(())
}
