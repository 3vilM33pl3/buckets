use std::path::{Path, PathBuf};
use std::fs;



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

    while let Some(parent) = current_path.parent() {
        let potential_target = parent.join(target_dir_name);
        if potential_target.is_dir() && fs::metadata(&potential_target).is_ok() {
            return Some(potential_target);
        }
        current_path = parent;
    }

    None
}

pub fn is_directory_empty(dir_path: &Path) -> bool {
    PathBuf::from(dir_path).read_dir().map(|mut i| i.next().is_none()).unwrap_or(false)
}

#[cfg(test)]
use tempfile::tempdir;
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;

    #[test]
    fn test_find_directory_in_parents() {
        let temp_dir = tempdir().unwrap();
        let target_dir_name = "target_directory";

        // Create a nested directory structure
        let nested_dir_path = temp_dir.path().join("a/b/c/d/e");
        create_dir_all(&nested_dir_path).unwrap();

        // Create the target directory
        let target_dir_path = temp_dir.path().join("a/target_directory");
        create_dir_all(&target_dir_path).unwrap();

        // Start the search from the deepest directory
        let start_path = nested_dir_path;

        // Perform the test
        let result = find_directory_in_parents(&start_path, target_dir_name);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), target_dir_path);
    }
    #[test]
    fn test_is_directory_empty() {
        let temp_dir = tempdir().unwrap();
        let empty_dir_path = temp_dir.path().join("empty_dir");
        create_dir_all(&empty_dir_path).unwrap();

        let result = is_directory_empty(&empty_dir_path);
        assert!(result);

        let non_empty_dir_path = temp_dir.path().join("non_empty_dir");
        create_dir_all(&non_empty_dir_path).unwrap();
        let file_path = non_empty_dir_path.join("file.txt");
        fs::File::create(&file_path).unwrap();

        let result = is_directory_empty(&non_empty_dir_path);
        assert!(!result);
    }
}


