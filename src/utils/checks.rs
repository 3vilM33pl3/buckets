use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

/// Searches for a directory with the given name in the parent directories.
///
/// # Arguments
///
/// * `start_path` - The path to start the search from.
/// * `target_dir_name` - The name of the directory to search for.
///
/// # Returns
///
/// Returns `Some(PathBuf)` containing the path to the found directory or `None` if not found.
pub fn find_directory_in_parents(start_path: &Path, target_dir_name: &str) -> Option<PathBuf> {
    let mut current_path = start_path;

    let potential_target = current_path.join(target_dir_name);
    if potential_target.is_dir() && fs::metadata(&potential_target).is_ok() {
        return Some(potential_target);
    }

    while let Some(parent) = current_path.parent() {
        let potential_target2 = parent.join(target_dir_name);
        if potential_target2.is_dir() && fs::metadata(&potential_target2).is_ok() {
            return Some(potential_target2);
        }
        current_path = parent;
    }

    None
}

/// Checks if the given directory is a valid bucket repository.
///
/// A valid bucket repository must contain a `.buckets` directory with a valid config file
/// and a database type indicator. The database type can come from:
/// - a `database_type` field in `.buckets/config` (current approach), or
/// - a legacy `database_type` file in `.buckets/` (backwards compat), or
/// - a legacy `postgres` directory in `.buckets/` (backwards compat).
pub fn is_valid_bucket_repo(dir_path: &Path) -> bool {
    debug!("{:?}", dir_path);
    // Find the .buckets directory
    let buckets_repo_path = find_directory_in_parents(dir_path, ".buckets");
    debug!("{:?}", buckets_repo_path);

    match buckets_repo_path {
        Some(path) => {
            // Check for a valid repository configuration
            if !is_valid_repo_config(&path) {
                debug!("config file is missing");
                return false;
            }

            // Check for database type: config field, legacy file, or postgres directory
            let db_type_path = path.join("database_type");
            let postgres_path = path.join("postgres");

            if db_type_path.exists() || postgres_path.exists() {
                debug!("Valid bucket repository structure found (legacy marker)");
                return true;
            }

            // Check if config contains database_type field
            let config_path = path.join("config");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(value) = content.parse::<toml::Value>() {
                    if value.get("database_type").is_some() {
                        debug!("Valid bucket repository structure found (config database_type)");
                        return true;
                    }
                }
            }

            debug!("No database type indicator found");
            false
        }
        None => false,
    }
}

// Note: is_valid_duckdb_database function removed as we're using PostgreSQL now
// Database validation is handled differently with PostgreSQL

pub fn is_valid_bucket(path: &Path) -> bool {
    let bucket_path = find_bucket_path(path);
    match bucket_path {
        Some(path) => has_valid_bucket_info(&path),
        None => false,
    }
}

fn has_valid_bucket_info(bucket_path: &Path) -> bool {
    let info_path = bucket_path.join(".b").join("info");
    if info_path.exists() && info_path.is_file() {
        return true;
    }
    false
}

pub fn is_valid_repo_config(dir_path: &Path) -> bool {
    let config_path = dir_path.join("config");
    if config_path.is_file() {
        return true;
    }
    false
}

#[allow(dead_code)]
pub fn is_valid_bucket_info(dir_path: &Path) -> bool {
    let config_path = dir_path.join("info");
    if config_path.is_file() {
        return true;
    }
    false
}

pub fn validate_path(path: &str) -> Result<PathBuf, String> {
    use crate::utils::security::validate_and_canonicalize_path;

    let path_buf = PathBuf::from(path);

    // Get the base directory for validation
    let base_dir = if path_buf.is_relative() {
        Some(CURRENT_DIR.with(|current_dir| current_dir.clone()))
    } else {
        None
    };

    // Use secure path validation
    let resolved_path = validate_and_canonicalize_path(&path_buf, base_dir.as_deref())
        .map_err(|e| e.to_string())?;

    if !resolved_path.exists() {
        Err(format!(
            "The path '{}' does not exist.",
            resolved_path.display()
        ))
    } else if !resolved_path.is_file() {
        Err(format!("'{}' is not a file.", resolved_path.display()))
    } else {
        Ok(resolved_path)
    }
}

use crate::utils::utils::find_bucket_path;
use crate::CURRENT_DIR;
#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};

    #[test]
    fn test_find_directory_in_parents() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let target_dir_name = "target_directory";

        // Create a nested directory structure
        let nested_dir_path = temp_dir.path().join("a/b/c/d/e");
        create_dir_all(&nested_dir_path).expect("Failed to create nested directory structure");

        // Create the target directory
        let target_dir_path = temp_dir.path().join("a/target_directory");
        create_dir_all(&target_dir_path).expect("Failed to create target directory");

        // Start the search from the deepest directory
        let start_path = nested_dir_path;

        // Perform the test
        let result = find_directory_in_parents(&start_path, target_dir_name);
        assert!(result.is_some());
        assert_eq!(result.expect(""), target_dir_path);
    }

    #[test]
    fn test_find_directory_in_current_dir() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let target_dir_name = "target_directory";

        // Create a nested directory structure
        let nested_dir_path = temp_dir.path().join("a");
        create_dir_all(&nested_dir_path).expect("Failed to create nested directory structure");

        // Create the target directory
        let target_dir_path = temp_dir.path().join("a/target_directory");
        create_dir_all(&target_dir_path).expect("Failed to create target directory");

        // Start the search from the deepest directory
        let start_path = nested_dir_path;

        // Perform the test
        let result = find_directory_in_parents(&start_path, target_dir_name);
        assert!(result.is_some());
        assert_eq!(result.expect(""), target_dir_path);
    }

    #[test]
    fn test_find_directory_in_parents_not_found() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let target_dir_name = "target_directory";

        // Create a nested directory structure
        let nested_dir_path = temp_dir.path().join("a/b/c/d/e");
        create_dir_all(&nested_dir_path).expect("Failed to create nested directory structure");

        // Start the search from the deepest directory
        let start_path = nested_dir_path;

        // Perform the test
        let result = find_directory_in_parents(&start_path, target_dir_name);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_valid_bucket_repo_empty_repo_dir() {
        // Create a temporary directory to simulate a bucket repository
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Case 1: No `.buckets` directory
        assert!(!is_valid_bucket_repo(temp_dir.path()));
    }

    #[test]
    fn test_is_valid_bucket_repo_empty_buckets_dir() {
        // Create a temporary directory to simulate a bucket repository
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");

        fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets directory");
        assert!(!is_valid_bucket_repo(temp_dir.path()));
    }

    #[test]
    fn test_is_valid_bucket_repo_no_db() {
        // Create a temporary directory to simulate a bucket repository
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        let config_path = buckets_dir.join("config");

        fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets directory");
        fs::File::create(&config_path).expect("Failed to create config file");
        assert!(!is_valid_bucket_repo(temp_dir.path()));
    }

    #[test]
    fn test_is_valid_bucket_repo_with_valid_repo() {
        // Create a temporary directory to simulate a bucket repository
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        let config_path = buckets_dir.join("config");

        fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets directory");
        fs::write(&config_path, "database_type = \"PostgreSQL\"\n")
            .expect("Failed to create config file");

        assert!(is_valid_bucket_repo(temp_dir.path()));
    }

    #[test]
    fn test_invalid_bucket_repo() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        assert!(!is_valid_bucket_repo(temp_dir.path()));
    }

    #[test]
    fn test_valid_repo_config() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config_file = temp_dir.path().join("config");
        File::create(&config_file).expect("Failed to create config file");

        assert!(is_valid_repo_config(temp_dir.path()));
    }

    #[test]
    fn test_invalid_repo_config() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        assert!(!is_valid_repo_config(temp_dir.path()));
    }
}
