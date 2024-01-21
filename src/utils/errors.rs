use std::fmt::{Display, Formatter};
use std::io;

pub enum BucketError {
    IoError(io::Error),
    Sqlite(rusqlite::Error),
    #[allow(dead_code)]
    BucketAlreadyExists,
    #[allow(dead_code)]
    NotInBucketRepo,
}

impl Display for BucketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketError::IoError(e) => write!(f, "IO Error: {}", e),
            BucketError::Sqlite(e) => write!(f, "Sqlite Error: {}", e),
            BucketError::BucketAlreadyExists => write!(f, "Bucket already exists"),
            BucketError::NotInBucketRepo => write!(f, "Not in a bucket repository"),
        }
    }
}

impl From<io::Error> for BucketError {
    fn from(error: io::Error) -> Self {
        BucketError::IoError(error)
    }
}

impl From<rusqlite::Error> for BucketError {
    fn from(error: rusqlite::Error) -> Self {
        BucketError::Sqlite(error)
    }
}