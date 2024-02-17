use crate::utils::errors::BucketError;
use std::fs;
use std::path::PathBuf;

pub fn delete_and_create_tmp_dir(bucket_path: &PathBuf) -> Result<PathBuf, BucketError> {
    let tmp_bucket_path = bucket_path.join(".b").join("tmp");
    fs::remove_dir_all(&tmp_bucket_path).unwrap_or_default();
    fs::create_dir_all(&tmp_bucket_path)?;
    Ok(tmp_bucket_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;
    use tempfile::tempdir;

    #[test]
    fn test_delete_and_create_tmp_dir() {
        let temp_dir = tempdir().unwrap();
        let bucket_tmp_path = temp_dir.path().join("bucket").join(".b").join("tmp");
        create_dir_all(&bucket_tmp_path).unwrap();

        let bucket_path = temp_dir.path().join("bucket");
        let result = delete_and_create_tmp_dir(&bucket_path);
        assert!(result.is_ok());
        assert!(bucket_path.join(".b").join("tmp").exists());
    }

    #[test]
    fn test_delete_and_create_tmp_dir_not_exist() {
        let temp_dir = tempdir().unwrap();
        let bucket_b_path = temp_dir.path().join("bucket").join(".b");
        create_dir_all(&bucket_b_path).unwrap();

        let bucket_path = temp_dir.path().join("bucket");
        let result = delete_and_create_tmp_dir(&bucket_path);
        assert!(result.is_ok());
        assert!(bucket_path.join(".b").join("tmp").exists());
    }
}
