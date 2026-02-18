// SPDX-License-Identifier: MIT

use buckets::errors::BucketError;
use buckets::postgres_db::get_database;
use uuid::Uuid;

use crate::models::{
    BucketRow, CommitRow, ExpectationRow, FileRow, GraphData, GraphEdge, GraphNode, PebbleRow,
    SemanticResult,
};

pub async fn fetch_buckets() -> Result<Vec<BucketRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT b.id, b.name, b.path, b.created_at,
                    COALESCE(cc.cnt, 0) AS commit_count,
                    COALESCE(ec.cnt, 0) AS expectation_count
             FROM buckets b
             LEFT JOIN (SELECT bucket_id, COUNT(*) AS cnt FROM commits GROUP BY bucket_id) cc
                 ON cc.bucket_id = b.id
             LEFT JOIN (SELECT bucket_id, COUNT(*) AS cnt FROM expectations GROUP BY bucket_id) ec
                 ON ec.bucket_id = b.id
             ORDER BY b.name",
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| BucketRow {
            id: r.get(0),
            name: r.get(1),
            path: r.get(2),
            created_at: r.get(3),
            commit_count: r.get(4),
            expectation_count: r.get(5),
        })
        .collect())
}

pub async fn fetch_bucket_by_id(id: Uuid) -> Result<Option<BucketRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT b.id, b.name, b.path, b.created_at,
                    COALESCE(cc.cnt, 0) AS commit_count,
                    COALESCE(ec.cnt, 0) AS expectation_count
             FROM buckets b
             LEFT JOIN (SELECT bucket_id, COUNT(*) AS cnt FROM commits GROUP BY bucket_id) cc
                 ON cc.bucket_id = b.id
             LEFT JOIN (SELECT bucket_id, COUNT(*) AS cnt FROM expectations GROUP BY bucket_id) ec
                 ON ec.bucket_id = b.id
             WHERE b.id = $1",
            &[&id],
        )
        .await?;

    Ok(rows.first().map(|r| BucketRow {
        id: r.get(0),
        name: r.get(1),
        path: r.get(2),
        created_at: r.get(3),
        commit_count: r.get(4),
        expectation_count: r.get(5),
    }))
}

pub async fn fetch_commits(bucket_id: Uuid) -> Result<Vec<CommitRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT c.id, b.name, c.message, c.created_at,
                    (SELECT COUNT(*) FROM files WHERE commit_id = c.id) AS file_count
             FROM commits c
             JOIN buckets b ON b.id = c.bucket_id
             WHERE c.bucket_id = $1
             ORDER BY c.created_at DESC",
            &[&bucket_id],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| CommitRow {
            id: r.get(0),
            bucket_name: r.get(1),
            message: r.get(2),
            created_at: r.get(3),
            file_count: r.get(4),
        })
        .collect())
}

pub async fn fetch_files(commit_id: Uuid) -> Result<Vec<FileRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT id, file_path, hash FROM files WHERE commit_id = $1 ORDER BY file_path",
            &[&commit_id],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| FileRow {
            id: r.get(0),
            file_path: r.get(1),
            hash: r.get(2),
        })
        .collect())
}

pub async fn fetch_expectations() -> Result<Vec<ExpectationRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT e.id, e.description, e.status, e.created_at,
                    b1.name AS source,
                    b2.name AS target
             FROM expectations e
             JOIN buckets b1 ON e.bucket_id = b1.id
             LEFT JOIN buckets b2 ON e.target_bucket_id = b2.id
             ORDER BY e.created_at DESC",
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| ExpectationRow {
            id: r.get(0),
            description: r.get(1),
            status: r.get(2),
            created_at: r.get(3),
            source_bucket: r.get(4),
            target_bucket: r.get(5),
        })
        .collect())
}

pub async fn fetch_expectations_for_bucket(
    bucket_id: Uuid,
) -> Result<Vec<ExpectationRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT e.id, e.description, e.status, e.created_at,
                    b1.name AS source,
                    b2.name AS target
             FROM expectations e
             JOIN buckets b1 ON e.bucket_id = b1.id
             LEFT JOIN buckets b2 ON e.target_bucket_id = b2.id
             WHERE e.bucket_id = $1 OR e.target_bucket_id = $1
             ORDER BY e.created_at DESC",
            &[&bucket_id],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| ExpectationRow {
            id: r.get(0),
            description: r.get(1),
            status: r.get(2),
            created_at: r.get(3),
            source_bucket: r.get(4),
            target_bucket: r.get(5),
        })
        .collect())
}

pub async fn fetch_pebbles() -> Result<Vec<PebbleRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT p.id, p.description, p.status, p.created_at,
                    b1.name AS origin,
                    b2.name AS current
             FROM pebbles p
             JOIN buckets b1 ON p.origin_bucket_id = b1.id
             JOIN buckets b2 ON p.current_bucket_id = b2.id
             ORDER BY p.created_at DESC",
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| PebbleRow {
            id: r.get(0),
            description: r.get(1),
            status: r.get(2),
            created_at: r.get(3),
            origin_bucket: r.get(4),
            current_bucket: r.get(5),
        })
        .collect())
}

pub async fn fetch_pebbles_for_bucket(bucket_id: Uuid) -> Result<Vec<PebbleRow>, BucketError> {
    let db = get_database().await?;
    let rows = db
        .query(
            "SELECT p.id, p.description, p.status, p.created_at,
                    b1.name AS origin,
                    b2.name AS current
             FROM pebbles p
             JOIN buckets b1 ON p.origin_bucket_id = b1.id
             JOIN buckets b2 ON p.current_bucket_id = b2.id
             WHERE p.current_bucket_id = $1 OR p.origin_bucket_id = $1
             ORDER BY p.created_at DESC",
            &[&bucket_id],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|r| PebbleRow {
            id: r.get(0),
            description: r.get(1),
            status: r.get(2),
            created_at: r.get(3),
            origin_bucket: r.get(4),
            current_bucket: r.get(5),
        })
        .collect())
}

pub async fn fetch_graph_data() -> Result<GraphData, BucketError> {
    let db = get_database().await?;

    let node_rows = db
        .query("SELECT id, name FROM buckets ORDER BY name", &[])
        .await?;
    let nodes: Vec<GraphNode> = node_rows
        .iter()
        .map(|r| GraphNode {
            id: r.get(0),
            name: r.get(1),
        })
        .collect();

    let edge_rows = db
        .query(
            "SELECT bucket_id, target_bucket_id, COUNT(*) AS cnt
             FROM expectations
             WHERE target_bucket_id IS NOT NULL
             GROUP BY bucket_id, target_bucket_id",
            &[],
        )
        .await?;
    let edges: Vec<GraphEdge> = edge_rows
        .iter()
        .map(|r| GraphEdge {
            from: r.get(0),
            to: r.get(1),
            count: r.get(2),
        })
        .collect();

    Ok(GraphData { nodes, edges })
}

pub async fn semantic_search(query: &str) -> Result<Vec<SemanticResult>, BucketError> {
    let query_owned = query.to_string();
    let embedding = tokio::task::spawn_blocking(move || {
        buckets::utils::embeddings::EmbeddingGenerator::generate(&query_owned)
    })
    .await
    .map_err(|e| BucketError::EmbeddingError(format!("Task join error: {e}")))??;

    let db = get_database().await?;
    let results =
        buckets::data::expectation::Expectation::find_similar(&db, &embedding, 20, 0.3).await?;

    let bucket_rows = db.query("SELECT id, name FROM buckets", &[]).await?;
    let bucket_names: std::collections::HashMap<uuid::Uuid, String> =
        bucket_rows.iter().map(|r| (r.get(0), r.get(1))).collect();

    Ok(results
        .into_iter()
        .map(|(exp, sim)| {
            let source = bucket_names
                .get(&exp.bucket_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let target = exp
                .target_bucket_id
                .and_then(|id| bucket_names.get(&id).cloned());
            SemanticResult {
                expectation: ExpectationRow {
                    id: exp.id,
                    description: exp.description,
                    status: exp.status,
                    source_bucket: source,
                    target_bucket: target,
                    created_at: exp.created_at,
                },
                similarity: sim,
            }
        })
        .collect())
}
