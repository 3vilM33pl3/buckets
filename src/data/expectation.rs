use crate::errors::BucketError;
use crate::postgres_db::DatabaseManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pgvector::Vector;
use serde::Deserializer;
use serde::Serializer;

fn serialize_vector<S>(vector: &Option<Vector>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match vector {
        Some(v) => serializer.serialize_some(&v.to_vec()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_vector<'de, D>(deserializer: D) -> Result<Option<Vector>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<f32>> = Option::deserialize(deserializer)?;
    Ok(v.map(Vector::from))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expectation {
    pub id: Uuid,
    pub bucket_id: Uuid,
    pub target_bucket_id: Option<Uuid>,
    pub description: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    #[serde(
        serialize_with = "serialize_vector",
        deserialize_with = "deserialize_vector",
        skip_serializing_if = "Option::is_none"
    )]
    pub embedding: Option<Vector>,
}

impl Expectation {
    pub async fn create(
        db: &DatabaseManager,
        bucket_id: Uuid,
        target_bucket_id: Option<Uuid>,
        description: String,
        embedding: Option<Vec<f32>>,
    ) -> Result<Self, BucketError> {
        let id = Uuid::new_v4();
        let status = "pending";
        let vector = embedding.map(Vector::from);

        db.execute(
            "INSERT INTO expectations (id, bucket_id, target_bucket_id, description, status, embedding) VALUES ($1, $2, $3, $4, $5, $6)",
            &[&id, &bucket_id, &target_bucket_id, &description, &status, &vector],
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
            embedding: vector,
        })
    }

    pub async fn find_similar(
        db: &DatabaseManager,
        embedding: &[f32],
        limit: i64,
        similarity_threshold: f64,
    ) -> Result<Vec<(Self, f64)>, BucketError> {
        let vector = Vector::from(embedding.to_vec());
        let rows = db
            .query(
                "SELECT id, bucket_id, target_bucket_id, description, status, created_at, embedding, 1 - (embedding <=> $1) as similarity 
                 FROM expectations 
                 WHERE 1 - (embedding <=> $1) > $2
                 ORDER BY embedding <=> $1 LIMIT $3",
                &[&vector, &similarity_threshold, &limit],
            )
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let similarity: f64 = row.get(7);
            let expectation = Expectation {
                id: row.get(0),
                bucket_id: row.get(1),
                target_bucket_id: row.get(2),
                description: row.get(3),
                status: row.get(4),
                created_at: row.get(5),
                embedding: row.get(6),
            };
            results.push((expectation, similarity));
        }
        Ok(results)
    }

    #[allow(dead_code)]
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
                embedding: None, // Optimization: skip retrieving embedding unless needed
            }))
        } else {
            Ok(None)
        }
    }
}
