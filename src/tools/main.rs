mod file_creator;

use std::env;
use std::fs;

fn create_test_repo() -> std::io::Result<()> {
    let path = env::current_dir()?.join("test_repo");
    if path.exists() && path.is_dir() {
        fs::remove_dir_all(&path)?;
    }
    fs::create_dir_all(&path)
}

fn main() {
    if let Err(e) = create_test_repo() {
        eprintln!("Failed to create test repository: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_test_repo() {
        let temp_dir = tempdir().unwrap();
        let test_repo = temp_dir.path().join("test_repo/test_subdir");
        fs::create_dir_all(&test_repo).unwrap();
        env::set_current_dir(&test_repo).unwrap();

        create_test_repo().expect("Failed to create test repo");

        let path = env::current_dir().unwrap().join("test_repo");
        assert!(path.exists() && path.is_dir());

        let path = env::current_dir().unwrap().join("test_repo/test_subdir");
        assert!(!path.exists());
    }
}
