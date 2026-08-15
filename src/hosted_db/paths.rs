use crate::ayb_db::models::DBType;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::engine_for;
use std::fs;
use std::future::Future;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;
use std::path::{Path, PathBuf};
use uuid::{timestamp::context::ContextV7, Timestamp, Uuid};

/// Canonicalize a database file path by canonicalizing its parent
/// directory and appending the filename. Works even if the file doesn't
/// exist yet (new databases before first write).
pub fn canonical_db_path(path: &Path) -> Result<PathBuf, AybError> {
    let parent = path.parent().ok_or(AybError::Other {
        message: format!("Cannot determine parent of: {}", path.display()),
    })?;
    let file_name = path.file_name().ok_or(AybError::Other {
        message: format!("Cannot determine filename of: {}", path.display()),
    })?;
    Ok(fs::canonicalize(parent)?.join(file_name))
}

pub const CURRENT: &str = "current";
pub const CURRENT_TMP: &str = "current.tmp";
const DATABASES: &str = "databases";
const SNAPSHOTS: &str = "snapshots";
const EXPORTS: &str = "exports";

/// Create and return a fresh
/// `{data_path}/{tree}/{entity_slug}/{database_slug}/{time_sortable_uuid}/`
/// directory.
///
/// Active databases, snapshots, and exports live in separate trees but
/// share this layout: each version gets its own UUID directory, so a new
/// copy can be assembled without disturbing the one in use. Databases
/// also get a directory rather than a bare file because they can span
/// several files (e.g. a SQLite database plus its write-ahead log).
fn new_uuid_path(
    tree: &str,
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    let uuid = Uuid::new_v7(Timestamp::now(ContextV7::new()));
    let path: PathBuf = [
        data_path,
        tree,
        entity_slug,
        database_slug,
        &uuid.to_string(),
    ]
    .iter()
    .collect();
    if let Err(e) = fs::create_dir_all(&path) {
        return Err(AybError::Other {
            message: format!("Unable to create {tree} path for {entity_slug}/{database_slug}: {e}"),
        });
    }

    Ok(fs::canonicalize(path)?)
}

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
/// `{entity_slug}/{database_slug}`.
pub fn new_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    new_uuid_path(DATABASES, entity_slug, database_slug, data_path)
}

/// Returns a path to a new database location (the file for the future
/// database inside a newly created directory) after creating the
/// directory. The database engine creates the actual file on first use.
pub fn instantiated_new_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    Ok(new_database_path(entity_slug, database_slug, data_path)?.join(database_slug))
}

pub fn current_database_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    // `current` is a symlink to the database directory containing the
    // most recently restored/created version of the database.
    // Canonicalize the directory (resolves the symlink) and append the
    // filename, since the file may not exist yet for new databases.
    let dir: PathBuf = [data_path, DATABASES, entity_slug, database_slug, CURRENT]
        .iter()
        .collect();

    let canonical_dir = fs::canonicalize(dir)?;
    Ok(canonical_dir.join(database_slug))
}

/// Returns a path for a new database snapshot directory for storing a
/// snapshot of `{entity_slug}/{database_slug}`.
pub fn database_snapshot_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    new_uuid_path(SNAPSHOTS, entity_slug, database_slug, data_path)
}

/// Returns a path for a new export directory, used to hold the copy of
/// `{entity_slug}/{database_slug}` that a download streams from. Unlike
/// databases and snapshots, an export directory is transient: it is
/// removed as soon as the response has a handle on the file.
pub fn database_export_path(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
) -> Result<PathBuf, AybError> {
    new_uuid_path(EXPORTS, entity_slug, database_slug, data_path)
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

/// Replace the contents of `{entity_slug}/{database_slug}` with a
/// database file produced by `write_into`.
///
/// `write_into` is handed a freshly created staging directory and must
/// write the new database to `{staging_dir}/{database_slug}` (the same
/// contract `SnapshotStorage::retrieve_snapshot` already follows). The
/// staged file is then validated against `db_type`, and only if that
/// succeeds does `current` get repointed at the staging directory.
///
/// Shared by snapshot restores and imports, which are the same operation
/// with different sources for the replacement bytes. Ordering the steps
/// as stage -> validate -> swap means the file that becomes the database
/// is exactly the file that passed validation: a corrupt download, a
/// truncated copy, or an upload of the wrong engine type all fail while
/// `current` still points at the old directory, leaving readers on the
/// previous database. On failure the staging directory is removed rather
/// than left behind.
pub async fn replace_current_database<F, Fut>(
    entity_slug: &str,
    database_slug: &str,
    data_path: &str,
    db_type: &DBType,
    daemon_registry: &DaemonRegistry,
    write_into: F,
) -> Result<(), AybError>
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: Future<Output = Result<(), AybError>>,
{
    let staging_dir = new_database_path(entity_slug, database_slug, data_path)?;
    let staged_path = staging_dir.join(database_slug);

    let staged = match write_into(staging_dir.clone()).await {
        Ok(()) => engine_for(db_type).validate(&staged_path),
        Err(err) => Err(err),
    };
    if let Err(err) = staged {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(err);
    }

    set_current_database_and_clean_up(&staging_dir, daemon_registry).await
}

/// Declares `new_path` as the new current path (by symlinking the
/// current path to it) and, if a previous database existed as the
/// current database, delete it.
pub async fn set_current_database_and_clean_up(
    new_path: &Path,
    daemon_registry: &DaemonRegistry,
) -> Result<(), AybError> {
    let mut current_db_path = pathbuf_to_parent(new_path)?;
    let mut current_tmp_db_path = current_db_path.clone();
    current_db_path.push(CURRENT);
    current_tmp_db_path.push(CURRENT_TMP);
    let previous_database_dir = fs::canonicalize(current_db_path.clone());

    // Extract database slug from the directory structure for later use
    let database_slug_dir = pathbuf_to_parent(new_path)?;
    let database_slug = pathbuf_to_file_name(&database_slug_dir)?;

    symlink_directory(&fs::canonicalize(new_path)?, &current_tmp_db_path.clone())?;
    // Why create a temporary current symlink and then rename it? This
    // is apparently how one overwrites a symlink. See
    // https://stackoverflow.com/questions/37345844/how-to-overwrite-a-symlink-in-go.
    fs::rename(current_tmp_db_path, current_db_path)?;

    // Shut down daemon and remove previous path if it existed.
    if let Ok(previous_database_dir) = previous_database_dir {
        // Daemons are registered with the full file path, not just the directory
        let mut previous_database_file_path = previous_database_dir.clone();
        previous_database_file_path.push(&database_slug);

        // Shut down the daemon for the old database path before deleting the directory
        daemon_registry
            .shut_down_daemon(&previous_database_file_path)
            .await?;
        fs::remove_dir_all(previous_database_dir)?;
    }

    Ok(())
}
