use crate::args::ListCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::Bucket;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::checks;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ListOutput {
    buckets: Vec<BucketListItem>,
}

#[derive(Serialize, Deserialize)]
pub struct BucketListItem {
    id: String,
    name: String,
    path: String,
}

/// List command to show all buckets
pub struct List {
    #[allow(dead_code)]
    args: ListCommand,
}

impl BucketCommand for List {
    type Args = ListCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        debug!("Listing buckets in directory: {:?}", current_dir);

        if !checks::is_valid_bucket_repo(&current_dir) {
            debug!("Not in a valid bucket repository");
            return Err(BucketError::NotInRepo);
        }

        info!("Querying buckets from database");

        let buckets = RuntimeManager::block_on(self.query_buckets_async())?;
        debug!("Found {} buckets", buckets.len());

        if self.args.shared.json {
            let bucket_items: Vec<BucketListItem> = buckets
                .iter()
                .map(|bucket| BucketListItem {
                    id: bucket.id.to_string(),
                    name: bucket.name.clone(),
                    path: bucket.relative_bucket_path.display().to_string(),
                })
                .collect();

            let output = ListOutput {
                buckets: bucket_items,
            };
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else {
            if buckets.is_empty() {
                println!("No buckets found");
            } else {
                println!("Buckets:");
                for bucket in &buckets {
                    println!(
                        "  {} - {} ({})",
                        bucket.name,
                        bucket.id,
                        bucket.relative_bucket_path.display()
                    );
                }
            }
        }

        Ok(())
    }
}

impl List {
    async fn query_buckets_async(&self) -> Result<Vec<Bucket>, BucketError> {
        let db = get_database().await?;

        let rows = db.query("SELECT id, name, path FROM buckets", &[]).await?;

        let mut buckets = Vec::new();
        for row in &rows {
            let uuid: Uuid = row.get(0);
            let name: String = row.get(1);
            let path_str: String = row.get(2);

            buckets.push(Bucket {
                id: uuid,
                name,
                relative_bucket_path: PathBuf::from(path_str),
            });
        }

        Ok(buckets)
    }
}
