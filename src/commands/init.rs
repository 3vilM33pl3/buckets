use crate::utils::checks;
use crate::utils::config::create_default_config;
use crate::utils::errors::BucketError;
use crate::utils::errors::BucketError::BucketAlreadyExists;
use rusqlite::Connection;
use std::path::Path;
use std::{env, fs};

pub fn execute(repo_name: &String) -> Result<(), BucketError> {
    println!("Initialising bucket repository");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(BucketError::IoError(e));
        }
    };

    match checks::find_bucket_repo(current_path.as_path()) {
        Some(_found_path) => {
            return Err(BucketError::InBucketRepo);
        }
        _ => {}
    }

    // Check if directory with the same name exists
    let repo_path = current_path.join(repo_name);
    if repo_path.exists() && repo_path.is_dir() {
        return Err(BucketAlreadyExists);
    }

    // Create the .buckets directory
    let init_dir_path = repo_path.join(".buckets");
    fs::create_dir_all(&init_dir_path)?;

    // Create the buckets.conf file
    create_default_config(init_dir_path.as_path());

    // Create the database
    create_database(init_dir_path.as_path())?;

    // let now = Utc::now();
    // file.write_fmt(format_args!("{}", now.to_rfc3339()))?;

    Ok(())
}

fn create_database(location: &Path) -> Result<(), rusqlite::Error> {
    let db_path = location.join("buckets.db");
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE buckets (
            id CHAR(36) PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE commits (
            id CHAR(36) PRIMARY KEY,
            bucket_id INTEGER NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT,
            FOREIGN KEY (bucket_id) REFERENCES buckets (id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER set_timestamp_after_insert
         AFTER INSERT ON commits
         BEGIN
             UPDATE commits SET created_at = CURRENT_TIMESTAMP WHERE rowid = NEW.rowid;
         END;",
        [],
    )?;

    conn.execute(
        "CREATE TABLE files (
            id CHAR(36) PRIMARY KEY,
            commit_id INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            hash TEXT NOT NULL,
            FOREIGN KEY (commit_id) REFERENCES commits (id),
            UNIQUE (commit_id, file_path, hash)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER AutoGenBucketsGUID
             AFTER INSERT ON buckets
             FOR EACH ROW
             WHEN (NEW.id IS NULL)
             BEGIN
               UPDATE buckets SET id = (select hex( randomblob(4)) || '-' || hex( randomblob(2))
                         || '-' || '4' || substr( hex( randomblob(2)), 2) || '-'
                         || substr('AB89', 1 + (abs(random()) % 4) , 1)  ||
                         substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)) ) WHERE rowid = NEW.rowid;
             END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER AutoGenCommitsGUID
             AFTER INSERT ON commits
             FOR EACH ROW
             WHEN (NEW.id IS NULL)
             BEGIN
               UPDATE commits SET id = (select hex( randomblob(4)) || '-' || hex( randomblob(2))
                         || '-' || '4' || substr( hex( randomblob(2)), 2) || '-'
                         || substr('AB89', 1 + (abs(random()) % 4) , 1)  ||
                         substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)) ) WHERE rowid = NEW.rowid;
             END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER AutoGenFilesGUID
             AFTER INSERT ON files
             FOR EACH ROW
             WHEN (NEW.id IS NULL)
             BEGIN
               UPDATE files SET id = (select hex( randomblob(4)) || '-' || hex( randomblob(2))
                         || '-' || '4' || substr( hex( randomblob(2)), 2) || '-'
                         || substr('AB89', 1 + (abs(random()) % 4) , 1)  ||
                         substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)) ) WHERE rowid = NEW.rowid;
             END;",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_create_database() -> Result<(), std::io::Error> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("buckets.db");

        create_database(&temp_dir.path()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error creating database: {}", e),
            )
        })?;

        let conn = Connection::open(&db_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error opening database: {}", e),
            )
        })?;

        // Verify that tables exist
        let tables = ["buckets", "commits", "files"];
        for table in tables.iter() {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='{}';",
                    table
                ))
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Error preparing statement: {}", e),
                    )
                })?;

            let mut rows = stmt.query([]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error querying statement: {}", e),
                )
            })?;

            assert!(rows.next().is_ok(), "Table {} does not exist", table);
        }

        Ok(())
    }

    #[test]
    fn test_uuid_auto_generate() -> Result<(), std::io::Error> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("buckets.db");

        create_database(&temp_dir.path()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error creating database: {}", e),
            )
        })?;
        let conn = rusqlite::Connection::open(db_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error opening database: {}", e),
            )
        })?;

        insert_buckets(&conn)?;

        struct BucketTable {
            id: String,
            _name: String,
            _path: String,
        }
        let mut stmt = conn
            .prepare("SELECT id, name, path FROM buckets")
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error preparing statement: {}", e),
                )
            })?;
        let bucket_iter = stmt
            .query_map([], |row| {
                Ok(BucketTable {
                    id: row.get(0)?,
                    _name: row.get(1)?,
                    _path: row.get(2)?,
                })
            })
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error querying statement: {}", e),
                )
            })?;

        for bucket in bucket_iter {
            let bucket = bucket.unwrap();
            assert!(Uuid::parse_str(bucket.id.as_str()).is_ok());
        }

        Ok(())
    }

    fn insert_buckets(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "INSERT INTO buckets (
                    id,
                    name,
                    path
                    )
                VALUES (
                    NULL,
                    'test_bucket1',
                    '/path_to_bucket2'
                )
                ",
            [],
        )
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error inserting into database: {}", e),
                )
            })?;

        conn.execute(
            "INSERT INTO buckets (
                    id,
                    name,
                    path
                    )
                VALUES (
                    NULL,
                    'test_bucket2',
                    '/path_to_bucket2'
                )
                ",
            [],
        )
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error inserting into database: {}", e),
                )
            })?;
        Ok(())
    }
}
