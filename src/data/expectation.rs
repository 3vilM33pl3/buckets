use crate::errors::BucketError;
use crate::postgres_db::DatabaseManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Expectation {
    pub id: Uuid,
    pub bucket_id: Uuid,
    pub target_bucket_id: Option<Uuid>,
    pub description: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

impl Expectation {
    pub async fn create(
        db: &DatabaseManager,
        bucket_id: Uuid,
        target_bucket_id: Option<Uuid>,
        description: String,
    ) -> Result<Self, BucketError> {
        let id = Uuid::new_v4();
        let status = "pending";

        db.execute(
            "INSERT INTO expectations (id, bucket_id, target_bucket_id, description, status) VALUES ($1, $2, $3, $4, $5)",
            &[&id, &bucket_id, &target_bucket_id, &description, &status],
        )
        .await?;

        // Fetch the created record to get the timestamp
        let row = db
            .query("SELECT created_at FROM expectations WHERE id = $1", &[&id])
            .await?
            .pop()
            .ok_or_else(|| {
                BucketError::DatabaseError("Failed to fetch created expectation".to_string())
            })?;

        Ok(Expectation {
            id,
            bucket_id,
            target_bucket_id,
            description,
            status: status.to_string(),
            created_at: row.get(0),
        })
    }

    pub async fn get_by_id(db: &DatabaseManager, id: Uuid) -> Result<Option<Self>, BucketError> {
        let rows = db
            .query(
                "SELECT id, bucket_id, target_bucket_id, description, status, created_at FROM expectations WHERE id = $1",
                &[&id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(Some(Expectation {
                id: row.get(0),
                bucket_id: row.get(1),
                target_bucket_id: row.get(2),
                description: row.get(3),
                status: row.get(4),
                created_at: row.get(5),
            }))
        } else {
            Ok(None)
        }
    }
}
