use std::path::PathBuf;

use crate::{
    data::bucket::{Bucket, BucketTrait},
    errors::BucketError,
    utils::{checks, utils::find_bucket_repo},
    CURRENT_DIR,
};

pub struct World {
    // Path to the working directory
    #[allow(dead_code)]
    pub work_dir: PathBuf,
    // The root directory of the repository
    #[allow(dead_code)]
    pub repo_root: PathBuf,
    // Path to the database file
    #[allow(dead_code)]
    pub repo_db_path: PathBuf,
    // The active bucket, None if no bucket is active
    #[allow(dead_code)]
    pub bucket: Option<Bucket>,
    // Verbose output
    #[allow(dead_code)]
    pub verbose: bool,
}

impl World {
    pub fn new(verbose: bool) -> Result<Self, BucketError> {
        let work_dir = CURRENT_DIR.with(|dir| dir.clone());

        if !checks::is_valid_bucket_repo(&work_dir) {
            return Err(BucketError::NotInRepo);
        }

        let repo_root = match find_bucket_repo(&work_dir) {
            Some(path) => path,
            None => return Err(BucketError::NotInRepo),
        };

        let repo_db_path = repo_root.join(".buckets").join("buckets.db");

        let bucket = Bucket::from_meta_data(&work_dir).ok();

        Ok(World {
            work_dir,
            repo_root,
            repo_db_path,
            bucket,
            verbose,
        })
    }
}
