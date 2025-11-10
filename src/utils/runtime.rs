use std::io::{Error, ErrorKind};

use once_cell::sync::Lazy;
use tokio::runtime::{Handle, Runtime};

use crate::errors::BucketError;

static RUNTIME: Lazy<Result<Runtime, std::io::Error>> =
    Lazy::new(|| tokio::runtime::Runtime::new());

pub struct RuntimeManager;

impl RuntimeManager {
    fn runtime() -> Result<&'static Runtime, BucketError> {
        match &*RUNTIME {
            Ok(runtime) => Ok(runtime),
            Err(err) => Err(Self::runtime_error(err)),
        }
    }

    fn runtime_error(err: &std::io::Error) -> BucketError {
        BucketError::IoError(Error::new(
            ErrorKind::Other,
            format!("Failed to create async runtime: {}", err),
        ))
    }

    pub fn handle() -> Result<Handle, BucketError> {
        let runtime = Self::runtime()?;
        Ok(runtime.handle().clone())
    }

    pub fn block_on<F, T>(future: F) -> Result<T, BucketError>
    where
        F: std::future::Future<Output = Result<T, BucketError>>,
    {
        let runtime = Self::runtime()?;
        runtime.block_on(future)
    }
}
