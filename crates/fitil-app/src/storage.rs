//! Where Fitil keeps its files.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fitil_market::SharedStore;
use fitil_store::Store;

use crate::error::AppError;

/// The candle cache, in the local (non-roaming) app data folder. The `FITIL_DATA_DIR`
/// environment variable puts it elsewhere, for tests and portable use.
pub fn cache_path() -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("FITIL_DATA_DIR") {
        return Some(PathBuf::from(folder).join("cache.sqlite"));
    }
    let folder = if cfg!(target_os = "linux") {
        "fitil"
    } else {
        "Fitil"
    };
    directories::BaseDirs::new().map(|dirs| dirs.data_local_dir().join(folder).join("cache.sqlite"))
}

pub fn open_cache() -> Result<SharedStore, AppError> {
    let path = cache_path().ok_or_else(|| AppError::Storage("no data folder".into()))?;
    Ok(Arc::new(Mutex::new(Store::open(&path)?)))
}
