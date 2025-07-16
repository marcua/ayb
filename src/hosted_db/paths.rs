use crate::error::AybError;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;
use std::path::{Path, PathBuf};
use uuid::{timestamp::context::ContextV7, Timestamp, Uuid};

pub const CURRENT: &str = "current";
pub const CURRENT_TMP: &str = "current.tmp";
const DATABASES: &str = "databases";
const SNAPSHOTS: &str = "snapshots";

pub fn database_parent_path(data_path: &str, create_path: bool) -> Result<PathBuf, AybError> {
    let path: PathBuf = [data_path, DATABASES].iter().collect();
    if create_path {
        if let Err(e) = fs::create_dir_all(&path) {
            return Err(AybError::Other {
                message: format!(
                    "Unable to create database parent path {}: {}",
                    path.display(),
                    e
                ),
            });
        }
    }
    Ok(fs::canonicalize(path)?)
}

/// Returns a path for a new database directory for storing
/// `{entity_slug}/{database_slug}`. The format for this path is
/// `{data_path}/databases/{entity_slug}/{database_slug}/{time_sortable_uuid}/`.
pub fn new_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    let uuid = Uuid::new_v7(Timestamp::now(ContextV7::new()));
    // We place each database in its own directory because databases
    // might span multiple files (e.g, the SQLite database file as
    // well as a journal/write-ahead log).
    let path: PathBuf = [
        data_path,
        DATABASES,
        entity_slug,
        database_slug,
        &uuid.to_string(),
    ]
    .iter()
    .collect();
    if let Err(e) = fs::create_dir_all(&path) {
        return Err(AybError::Other {
            message: format!(
                "Unable to create database path for {entity_slug}/{database_slug}: {e}"
            ),
        });
    }

    Ok(path)
}

/// Returns a path to a new database location (the file for the future
/// database inside a newly created directory) after creating a
/// directory and empty file in the future location of that database.
pub fn instantiated_new_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    let mut path = new_database_path(entity_slug, database_slug, data_path)?;
    path.push(database_slug);
    if !path.exists() {
        fs::File::create(path.clone())?;
    }

    Ok(fs::canonicalize(path)?)
}

pub fn current_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    // `current` is a symlink to the database directory containing the
    // most recently restored/created version of the database.
    let path: PathBuf = [
        data_path,
        DATABASES,
        entity_slug,
        database_slug,
        CURRENT,
        database_slug,
    ]
    .iter()
    .collect();

    Ok(fs::canonicalize(path)?)
}

/// Returns a path for a new database snapshot directory for storing a
/// snapshot of `{entity_slug}/{database_slug}`. The format for this
/// path is
/// `{data_path}/snapshots/{entity_slug}/{database_slug}/{time_sortable_uuid}/`.
pub fn database_snapshot_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    let uuid = Uuid::new_v7(Timestamp::now(ContextV7::new()));
    let path: PathBuf = [
        data_path,
        SNAPSHOTS,
        entity_slug,
        database_slug,
        &uuid.to_string(),
    ]
    .iter()
    .collect();
    if let Err(e) = fs::create_dir_all(&path) {
        return Err(AybError::Other {
            message: format!(
                "Unable to create snapshot path for {entity_slug}/{database_slug}: {e}"
            ),
        });
    }

    Ok(fs::canonicalize(path)?)
}

pub fn pathbuf_to_file_name(path: &Path) -> Result<String, AybError> {
    Ok(path
        .file_name()
        .ok_or(AybError::Other {
            message: format!("Could not parse file name from path: {}", path.display()),
        })?
        .to_str()
        .ok_or(AybError::Other {
            message: format!("Could not convert path to string: {}", path.display()),
        })?
        .to_string())
}

pub fn pathbuf_to_parent(path: &Path) -> Result<PathBuf, AybError> {
    Ok(path
        .parent()
        .ok_or(AybError::Other {
            message: format!("Unable to find parent directory of {}", path.display()),
        })?
        .to_owned())
}

#[cfg(unix)]
fn symlink_directory(original: &Path, link: &Path) -> Result<(), AybError> {
    symlink(original, link)?;
    Ok(())
}

#[cfg(windows)]
fn symlink_directory(original: &Path, link: &Path) -> Result<(), AybError> {
    symlink_dir(original, link)?;
    Ok(())
}

/// Declares `new_path` as the new current path (by symlinking the
/// current path to it) and, if a previous database existed as the
/// current database, delete it.
pub fn set_current_database_and_clean_up(new_path: &Path) -> Result<(), AybError> {
    let mut current_db_path = pathbuf_to_parent(new_path)?;
    let mut current_tmp_db_path = current_db_path.clone();
    current_db_path.push(CURRENT);
    current_tmp_db_path.push(CURRENT_TMP);
    let previous_database_path = fs::canonicalize(current_db_path.clone());

    symlink_directory(&fs::canonicalize(new_path)?, &current_tmp_db_path.clone())?;
    // Why create a temporary current symlink and then rename it? This
    // is apparently how one overwrites a symlink. See
    // https://stackoverflow.com/questions/37345844/how-to-overwrite-a-symlink-in-go.
    fs::rename(current_tmp_db_path, current_db_path)?;

    // Remove previous path if it existed.
    if let Ok(previous_database_path) = previous_database_path {
        fs::remove_dir_all(previous_database_path)?;
    }

    Ok(())
}
