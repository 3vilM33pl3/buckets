use crate::errors::BucketError;
use crate::postgres_db::DatabaseManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pebble {
    pub id: Uuid,
    pub description: String,
    pub origin_bucket_id: Uuid,
    pub current_bucket_id: Uuid,
    pub original_pebble_id: Option<Uuid>,
    pub created_via_expectation_id: Uuid,
    pub status: String,
    pub created_at: NaiveDateTime,
}

impl Pebble {
    pub async fn create(
        db: &DatabaseManager,
        description: String,
        origin_bucket_id: Uuid,
        current_bucket_id: Uuid,
        original_pebble_id: Option<Uuid>,
        created_via_expectation_id: Uuid,
    ) -> Result<Self, BucketError> {
        let id = Uuid::new_v4();
        let status = "active";

        db.execute(
            "INSERT INTO pebbles (id, description, origin_bucket_id, current_bucket_id, original_pebble_id, created_via_expectation_id, status) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &description, &origin_bucket_id, &current_bucket_id, &original_pebble_id, &created_via_expectation_id, &status],
        )
        .await?;

        // Fetch the created record to get the timestamp
        let row = db
            .query("SELECT created_at FROM pebbles WHERE id = $1", &[&id])
            .await?
            .pop()
            .ok_or_else(|| {
                BucketError::DatabaseError("Failed to fetch created pebble".to_string())
            })?;

        Ok(Pebble {
            id,
            description,
            origin_bucket_id,
            current_bucket_id,
            original_pebble_id,
            created_via_expectation_id,
            status: status.to_string(),
            created_at: row.get(0),
        })
    }

    pub async fn get_active_in_bucket(
        db: &DatabaseManager,
        bucket_id: Uuid,
    ) -> Result<Vec<Self>, BucketError> {
        let rows = db
            .query(
                "SELECT id, description, origin_bucket_id, current_bucket_id, original_pebble_id, created_via_expectation_id, status, created_at FROM pebbles WHERE current_bucket_id = $1 AND status = 'active'",
                &[&bucket_id],
            )
            .await?;

        let mut pebbles = Vec::new();
        for row in rows {
            pebbles.push(Pebble {
                id: row.get(0),
                description: row.get(1),
                origin_bucket_id: row.get(2),
                current_bucket_id: row.get(3),
                original_pebble_id: row.get(4),
                created_via_expectation_id: row.get(5),
                status: row.get(6),
                created_at: row.get(7),
            });
        }
        Ok(pebbles)
    }

    pub async fn resolve(db: &DatabaseManager, id: Uuid) -> Result<(), BucketError> {
        db.execute(
            "UPDATE pebbles SET status = 'resolved' WHERE id = $1",
            &[&id],
        )
        .await?;
        Ok(())
    }
}
