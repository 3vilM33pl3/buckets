use crate::args::CreateCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::checks;
use crate::utils::checks::{find_directory_in_parents, is_valid_bucket};
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

/// Create a new bucket
pub struct Create {
    args: CreateCommand,
}

impl BucketCommand for Create {
    type Args = CreateCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let bucket_name = &self.args.bucket_name;

        self.checks(bucket_name)?;

        let bucket_path = CURRENT_DIR.with(|dir| dir.join(bucket_name));
        std::fs::create_dir_all(bucket_path.join(".b").join("storage"))?;

        let buckets_repo_path = find_directory_in_parents(&bucket_path, ".buckets")
            .ok_or_else(|| BucketError::NotInRepo)?;
        let relative_path = match bucket_path.strip_prefix(
            buckets_repo_path
                .parent()
                .ok_or_else(|| BucketError::NotInRepo)?,
        ) {
            Ok(x) => x,
            Err(_) => {
                return Err(BucketError::IoError(std::io::Error::other(
                    "Error stripping prefix",
                )))
            }
        }
        .to_path_buf();

        let bucket_id = RuntimeManager::block_on(async {
            let db = get_database().await?;

            let path_str = relative_path
                .to_str()
                .ok_or_else(|| BucketError::from("Invalid path string"))?;

            let params: Vec<&(dyn ToSql + Sync)> = vec![bucket_name, &path_str];

            // Insert and get the new bucket ID in one query
            let rows = db.query(
                "INSERT INTO buckets (id, name, path) VALUES (uuid_generate_v4(), $1, $2) RETURNING id",
                &params,
            ).await.map_err(|e| {
                BucketError::from(format!("Error inserting into database: {}", e).as_str())
            })?;

            if let Some(row) = rows.first() {
                let id: Uuid = row.get(0);
                Ok(id)
            } else {
                Err(BucketError::from("Failed to get bucket ID from insert"))
            }
        })?;

        let bucket = Bucket::default(bucket_id, bucket_name, &relative_path);
        bucket.write_bucket_info()?;

        Ok(())
    }
}

impl Create {
    fn checks(&self, bucket_name: &str) -> Result<(), BucketError> {
        let bucket_location = CURRENT_DIR.with(|dir| dir.join(bucket_name));
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());

        // Check if in valid buckets repository
        if !checks::is_valid_bucket_repo(&current_dir) {
            return Err(BucketError::NotInRepo);
        }

        // Validate bucket name
        if bucket_name.is_empty() {
            return Err(BucketError::InvalidBucketName(
                "cannot be empty".to_string(),
            ));
        }

        if bucket_name == "." || bucket_name == ".." {
            return Err(BucketError::InvalidBucketName(
                "cannot be '.' or '..'".to_string(),
            ));
        }

        if bucket_name.contains('/') || bucket_name.contains('\\') {
            return Err(BucketError::InvalidBucketName(
                "cannot contain path separators".to_string(),
            ));
        }

        if bucket_name.contains('\0') {
            return Err(BucketError::InvalidBucketName(
                "cannot contain null characters".to_string(),
            ));
        }

        // Additional security checks for dangerous characters
        if bucket_name.chars().any(|c| c.is_control()) {
            return Err(BucketError::InvalidBucketName(
                "cannot contain control characters".to_string(),
            ));
        }

        // Check for reserved names (Windows compatibility)
        let upper_name = bucket_name.to_uppercase();
        const RESERVED_NAMES: &[&str] = &["CON", "PRN", "AUX", "NUL"];
        if RESERVED_NAMES.contains(&upper_name.as_str()) {
            return Err(BucketError::InvalidBucketName(
                "cannot use reserved system names".to_string(),
            ));
        }

        if bucket_name.len() > 255 {
            return Err(BucketError::InvalidBucketName(
                "too long (maximum 255 characters)".to_string(),
            ));
        }

        if bucket_location.exists() {
            if bucket_location.is_dir() && is_valid_bucket(&bucket_location) {
                return Err(BucketError::BucketAlreadyExists);
            } else if bucket_location.is_file() {
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "File with the same name already exists",
                )));
            } else if bucket_location.is_dir() {
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Directory already exists",
                )));
            } else {
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unknown error",
                )));
            }
        }

        Ok(())
    }
}
