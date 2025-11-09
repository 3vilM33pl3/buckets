use crate::args::StatusCommand;
use crate::commands::commit::Commit;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::commit::CommitStatus;
use crate::errors::BucketError;
use crate::utils::checks;
use crate::utils::config::RepositoryConfig;
use crate::utils::utils::find_bucket_path;
use crate::CURRENT_DIR;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Error, ErrorKind};

#[derive(Serialize, Deserialize, Debug)]
pub struct BucketStatusOutput {
    pub files: Vec<FileStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileStatus {
    pub name: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RepositoryStatusOutput {
    pub config: String,
    pub bucket_count: usize,
    pub buckets: Vec<BucketInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BucketInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Show status of the current bucket or repository
pub struct Status {
    #[allow(dead_code)]
    args: StatusCommand,
}

impl BucketCommand for Status {
    type Args = StatusCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        debug!("Checking status in directory: {:?}", current_dir);

        if !checks::is_valid_bucket_repo(&current_dir) {
            debug!("Not in a valid bucket repository");
            return Err(BucketError::NotInRepo);
        }

        if !checks::is_valid_bucket(&current_dir) {
            info!("Showing repository status");
            self.repository_status()
        } else {
            info!("Showing bucket status");
            let bucket_path =
                find_bucket_path(&current_dir).ok_or_else(|| BucketError::NotAValidBucket)?;
            debug!("Found bucket path: {:?}", bucket_path);

            let bucket = match Bucket::from_meta_data(&bucket_path) {
                Ok(bucket) => bucket,
                Err(e) => {
                    error!("Error reading bucket info: {}", e);
                    return Err(e);
                }
            };
            self.bucket_status(&bucket)
        }?;

        Ok(())
    }
}

impl Status {
    fn bucket_status(&self, bucket: &Bucket) -> Result<(), BucketError> {
        // Read the bucket's metadata
        let bucket = Bucket::from_meta_data(&bucket.get_full_bucket_path()?)?;
        let bucket_files = bucket.list_files_with_metadata_in_bucket()?;

        if bucket_files.files.is_empty() {
            if self.args.shared.json {
                let output = BucketStatusOutput { files: vec![] };
                match serde_json::to_string_pretty(&output) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error serializing to JSON: {}", e),
                }
            } else {
                println!("No files in bucket");
            }
            return Ok(());
        }

        // Create async runtime for database operations
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            BucketError::from(format!("Failed to create async runtime: {}", e).as_str())
        })?;

        let file_statuses = match rt.block_on(Commit::load_last_commit_async(bucket.id)) {
            Ok(None) => bucket_files
                .files
                .iter()
                .map(|file| FileStatus {
                    name: file.name.clone(),
                    status: file.status.to_string(),
                })
                .collect::<Vec<FileStatus>>(),
            Ok(Some(previous_commit)) => match bucket_files.compare(&previous_commit) {
                Some(changes) => changes
                    .iter()
                    .map(|change| FileStatus {
                        name: change.name.clone(),
                        status: change.status.to_string(),
                    })
                    .collect::<Vec<FileStatus>>(),
                None => bucket_files
                    .files
                    .iter()
                    .map(|file| FileStatus {
                        name: file.name.clone(),
                        status: CommitStatus::Committed.to_string(),
                    })
                    .collect::<Vec<FileStatus>>(),
            },
            Err(_) => {
                error!("Failed to load previous commit.");
                return Err(BucketError::from(Error::new(
                    ErrorKind::Other,
                    "Failed to load previous commit.",
                )));
            }
        };

        if self.args.shared.json {
            let output = BucketStatusOutput {
                files: file_statuses,
            };
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else {
            for file in &file_statuses {
                println!("{}:    {}", file.status, file.name);
            }
        }

        Ok(())
    }

    fn repository_status(&self) -> Result<(), BucketError> {
        let current_dir = env::current_dir().map_err(|e| {
            BucketError::from(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to get current directory: {}", e),
            ))
        })?;
        let repo_config = RepositoryConfig::from_file(current_dir)?;

        // Create async runtime for database operations
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            BucketError::from(format!("Failed to create async runtime: {}", e).as_str())
        })?;

        let buckets = rt.block_on(self.query_buckets_async())?;

        if self.args.shared.json {
            let bucket_infos: Vec<BucketInfo> = buckets
                .iter()
                .map(|bucket| BucketInfo {
                    id: bucket.id.to_string(),
                    name: bucket.name.clone(),
                    path: bucket.relative_bucket_path.display().to_string(),
                })
                .collect();

            let output = RepositoryStatusOutput {
                config: format!("{:?}", repo_config),
                bucket_count: buckets.len(),
                buckets: bucket_infos,
            };
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else {
            println!("Repository config: {:?}", repo_config);
            println!("Number of buckets: {:?}", buckets.len());
            println!("Buckets: {:?}", buckets);
        }
        Ok(())
    }

    async fn query_buckets_async(&self) -> Result<Vec<Bucket>, BucketError> {
        let db = crate::postgres_db::get_database().await?;

        let rows = db.query("SELECT id, name, path FROM buckets", &[]).await?;

        let mut buckets = Vec::new();
        for row in &rows {
            let uuid_str: String = row.get(0);
            let name: String = row.get(1);
            let path_str: String = row.get(2);

            let uuid = uuid::Uuid::parse_str(&uuid_str)
                .map_err(|e| BucketError::InvalidData(e.to_string()))?;

            buckets.push(Bucket {
                id: uuid,
                name,
                relative_bucket_path: std::path::PathBuf::from(path_str),
            });
        }

        Ok(buckets)
    }
}
