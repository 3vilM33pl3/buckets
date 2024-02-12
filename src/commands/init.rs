use crate::utils::checks;
use crate::utils::config::create_default_config;
use std::{env, fs};
use std::path::Path;
use rusqlite::{Connection};
use crate::utils::errors::BucketError;
use crate::utils::errors::BucketError::BucketAlreadyExists;

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
        },
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
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE commits (
            id INTEGER PRIMARY KEY,
            bucket_id INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (bucket_id) REFERENCES buckets (id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE files (
            id INTEGER PRIMARY KEY,
            commit_id INTEGER NOT NULL,
            md5 TEXT NOT NULL,
            size INTEGER NOT NULL,
            FOREIGN KEY (commit_id) REFERENCES commits (id)
        )",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
            let mut stmt = conn.prepare(&format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{}';", table)).map_err(|e| {
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
}
