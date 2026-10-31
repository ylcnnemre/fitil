//! Where Fitil keeps its files.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fitil_market::{SharedStore, SharedUserData};
use fitil_store::{Store, UserData};

use crate::error::AppError;

/// The candle cache, in the local (non-roaming) app data folder: it can grow large and
/// can always be downloaded again.
pub fn cache_path() -> Option<PathBuf> {
    data_file(Folder::Local, "cache.sqlite")
}

/// What the user made, such as indicator templates, in the roaming app data folder, so
/// that it follows the user on networks with roaming profiles.
pub fn user_data_path() -> Option<PathBuf> {
    data_file(Folder::Roaming, "user.sqlite")
}

enum Folder {
    Local,
    Roaming,
}

/// The file `name` in Fitil's data folder. The `FITIL_DATA_DIR` environment variable
/// puts every file elsewhere, for tests and portable use.
fn data_file(folder: Folder, name: &str) -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("FITIL_DATA_DIR") {
        return Some(PathBuf::from(folder).join(name));
    }
    let app = if cfg!(target_os = "linux") {
        "fitil"
    } else {
        "Fitil"
    };
    let dirs = directories::BaseDirs::new()?;
    let base = match folder {
        Folder::Local => dirs.data_local_dir(),
        Folder::Roaming => dirs.data_dir(),
    };
    Some(base.join(app).join(name))
}

pub fn open_cache() -> Result<SharedStore, AppError> {
    let path = cache_path().ok_or_else(|| AppError::Storage("no data folder".into()))?;
    Ok(Arc::new(Mutex::new(Store::open(&path)?)))
}

pub fn open_user_data() -> Result<SharedUserData, AppError> {
    let path = user_data_path().ok_or_else(|| AppError::UserData("no data folder".into()))?;
    let user = UserData::open(&path).map_err(|error| AppError::UserData(error.to_string()))?;
    Ok(Arc::new(Mutex::new(user)))
}
