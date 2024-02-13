use crate::error::AybError;
use std::fs;
use std::path::{Path, PathBuf};

const DATABASES: &str = "databases";
const SNAPSHOTS: &str = "snapshots";

pub fn database_parent_path(data_path: &str) -> Result<PathBuf, AybError> {
    let path: PathBuf = [data_path, DATABASES].iter().collect();
    Ok(fs::canonicalize(path)?)
}

pub fn database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
    create_database: bool,
) -> Result<PathBuf, AybError> {
    let mut path: PathBuf = [data_path, DATABASES, entity_slug].iter().collect();
    if create_database {
        if let Err(e) = fs::create_dir_all(&path) {
            return Err(AybError::Other {
                message: format!("Unable to create entity path for {}: {}", entity_slug, e),
            });
        }
    }

    path.push(database_slug);

    if create_database && !path.exists() {
        fs::File::create(path.clone())?;
    }

    Ok(fs::canonicalize(path)?)
}

pub fn database_snapshot_path(
    entity_slug: &str,
    database_slug: &str,
    snapshot_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    let path: PathBuf = [
        data_path,
        SNAPSHOTS,
        entity_slug,
        database_slug,
        snapshot_slug,
    ]
    .iter()
    .collect();
    if let Err(e) = fs::create_dir_all(&path) {
        return Err(AybError::Other {
            message: format!(
                "Unable to create snapshot path for {}/{}: {}",
                entity_slug, database_slug, e
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
