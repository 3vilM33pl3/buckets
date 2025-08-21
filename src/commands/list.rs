use crate::args::ListCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::Bucket;
use crate::errors::BucketError;
use crate::utils::checks;
use crate::utils::utils::with_db_connection;
use crate::CURRENT_DIR;
use serde::{Deserialize, Serialize};

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

        if !checks::is_valid_bucket_repo(&current_dir) {
            return Err(BucketError::NotInRepo);
        }

        let buckets = self.query_buckets()?;
        
        if self.args.shared.json {
            let bucket_items: Vec<BucketListItem> = buckets.iter().map(|bucket| BucketListItem {
                id: bucket.id.to_string(),
                name: bucket.name.clone(),
                path: bucket.relative_bucket_path.display().to_string(),
            }).collect();
            
            let output = ListOutput { buckets: bucket_items };
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
                    println!("  {} - {} ({})", bucket.name, bucket.id, bucket.relative_bucket_path.display());
                }
            }
        }
        
        Ok(())
    }
}

impl List {
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
                buckets.push(bucket.map_err(BucketError::from)?);
            }

            Ok(buckets)
        })
    }
}
