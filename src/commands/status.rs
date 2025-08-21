use crate::args::StatusCommand;
use crate::commands::commit::Commit;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::errors::BucketError;
use crate::utils::checks;
use crate::utils::config::RepositoryConfig;
use crate::utils::utils::{find_bucket_path, with_db_connection};
use crate::CURRENT_DIR;
use log::error;
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

        if !checks::is_valid_bucket_repo(&current_dir) {
            return Err(BucketError::NotInRepo);
        }

        if !checks::is_valid_bucket(&current_dir) {
            self.repository_status()
        } else {
            let bucket_path =
                find_bucket_path(&current_dir).ok_or_else(|| BucketError::NotAValidBucket)?;

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

        let file_statuses = match Commit::load_last_commit(bucket.id) {
            Ok(None) => {
                bucket_files.files.iter().map(|file| FileStatus {
                    name: file.name.clone(),
                    status: file.status.to_string(),
                }).collect::<Vec<FileStatus>>()
            }
            Ok(Some(previous_commit)) => {
                let changes = bucket_files
                    .compare(&previous_commit)
                    .ok_or_else(|| BucketError::from("Failed to compare files."))?;
                changes.iter().map(|change| FileStatus {
                    name: change.name.clone(),
                    status: change.status.to_string(),
                }).collect::<Vec<FileStatus>>()
            }
            Err(_) => {
                error!("Failed to load previous commit.");
                return Err(BucketError::from(Error::new(
                    ErrorKind::Other,
                    "Failed to load previous commit.",
                )));
            }
        };

        if self.args.shared.json {
            let output = BucketStatusOutput { files: file_statuses };
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
        let buckets = self.query_buckets().map_err(|e| BucketError::from(e))?;
        
        if self.args.shared.json {
            let bucket_infos: Vec<BucketInfo> = buckets.iter().map(|bucket| BucketInfo {
                id: bucket.id.to_string(),
                name: bucket.name.clone(),
                path: bucket.relative_bucket_path.display().to_string(),
            }).collect();
            
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

    fn query_buckets(&self) -> Result<Vec<Bucket>, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT id, name, path FROM buckets")?;
            let bucket_iter = stmt
                .query_map([], |row| {
                    let uuid_str: String = row.get(0)?;
                    let path_str: String = row.get(2)?;
                    let uuid = uuid::Uuid::parse_str(&uuid_str)
                        .map_err(|e| BucketError::InvalidData(e.to_string()))?;
                    Ok(Bucket {
                        id: uuid,
                        name: row.get(1)?,
                        relative_bucket_path: std::path::PathBuf::from(path_str),
                    })
                })
                .map_err(BucketError::from)?;
            let mut buckets = Vec::new();
            for bucket in bucket_iter {
                buckets.push(bucket.map_err(BucketError::from)?); // Ensure all errors are converted properly
            }

            Ok(buckets)
        })
    }
}
