pub mod models;
pub mod storage;

use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::hosted_db::paths::{
    current_database_path, database_parent_path, database_snapshot_path, pathbuf_to_file_name,
    pathbuf_to_parent,
};
use crate::hosted_db::sqlite::query_sqlite;
use crate::server::config::{AybConfig, SqliteSnapshotMethod};
use crate::server::snapshots::models::{Snapshot, SnapshotType};
use crate::server::snapshots::storage::SnapshotStorage;
use go_parse_duration::parse_duration;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use walkdir::WalkDir;

pub async fn schedule_periodic_snapshots(
    config: AybConfig,
    ayb_db: Box<dyn AybDb>,
) -> Result<(), AybError> {
    if let Some(ref snapshot_config) = config.snapshots {
        if let Some(ref automation_config) = snapshot_config.automation {
            let scheduler = JobScheduler::new().await?;
            // TODO(marcua): Consider something better than
            // try_into/unwrap. The problem is that `parse_duration`
            // produces an i64 and `from_nanos` expects u64.
            let duration = Duration::from_nanos(
                parse_duration(&automation_config.interval)?
                    .try_into()
                    .unwrap(),
            );
            scheduler
                .add(Job::new_repeated_async(duration, move |_, _| {
                    let config = config.clone();
                    let ayb_db = ayb_db.clone();
                    Box::pin(async move {
                        if let Some(err) = create_snapshots(&config.clone(), &ayb_db.clone())
                            .await
                            .err()
                        {
                            eprintln!("Unable to walk database directory for snapshots: {}", err);
                        }
                    })
                })?)
                .await?;
            scheduler.shutdown_on_ctrl_c();

            scheduler.start().await?;
        }
    }
    Ok(())
}

// TODO(marcua): Figure how how to avoid this Clippy ignore and the
// one on snapshot_database. If I remove the Box, I get an
// unimplemented trait compiler error, but if I keep it, I get a
// Clippy warning.
#[allow(clippy::borrowed_box)]
async fn create_snapshots(config: &AybConfig, ayb_db: &Box<dyn AybDb>) -> Result<(), AybError> {
    // Walk the data path for entity slugs, database slugs
    println!("Creating snapshots...");
    let mut visited: HashSet<String> = HashSet::new();
    for entry in WalkDir::new(database_parent_path(&config.data_path, true).unwrap())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir() && !e.path_is_symlink())
    {
        let path = entry.path();
        if let Some(err) = snapshot_database(config, ayb_db, path, &mut visited)
            .await
            .err()
        {
            eprintln!("Unable to snapshot database {}: {}", path.display(), err);
        }
    }

    Ok(())
}

#[allow(clippy::borrowed_box)]
pub async fn snapshot_database(
    config: &AybConfig,
    ayb_db: &Box<dyn AybDb>,
    path: &Path,
    visited: &mut HashSet<String>,
) -> Result<(), AybError> {
    // TODO(marcua): Replace printlns with some structured logging or
    // tracing library.
    println!("Trying to back up {}", path.display());
    let entity_slug = pathbuf_to_file_name(&pathbuf_to_parent(&pathbuf_to_parent(
        &pathbuf_to_parent(path)?,
    )?)?)?;
    let database_slug = pathbuf_to_file_name(path)?;
    let visited_path = format!("{}/{}", entity_slug, database_slug);
    if visited.contains(&visited_path) {
        // We only need to snapshot each database once per run, but we
        // might encounter multiple versions of the database. Return
        // early if we've already taken a backup.
        return Ok(());
    }
    visited.insert(visited_path);
    if config.snapshots.is_none() {
        return Err(AybError::SnapshotError {
            message: "No snapshot config found".to_string(),
        });
    }
    let snapshot_config = config.snapshots.as_ref().unwrap();

    match ayb_db.get_database(&entity_slug, &database_slug).await {
        Ok(_db) => {
            // TODO(marcua): Implement hashing. `.sha3sum --schema` is
            // only available at the SQLite command line since it's a
            // dot command.
            let db_path = current_database_path(&entity_slug, &database_slug, &config.data_path)?;
            // TODO(marcua): Do better than "temporary"
            // by creating a tmpdir.
            let mut snapshot_path = database_snapshot_path(
                &entity_slug,
                &database_slug,
                "temporary",
                &config.data_path,
            )?;
            snapshot_path.push(&database_slug);
            // Try to remove the file if it already exists, but don't fail if it doesn't.
            fs::remove_file(&snapshot_path).ok();
            let backup_query = match snapshot_config.sqlite_method {
                // TODO(marcua): Figure out dot commands to make .backup work
                SqliteSnapshotMethod::Backup => {
                    return Err(AybError::SnapshotError {
                        message: "Backup requires dot commands, which are not yet supported"
                            .to_string(),
                    })
                }
                SqliteSnapshotMethod::Vacuum => {
                    format!("VACUUM INTO \"{}\"", snapshot_path.display())
                }
            };
            let result = query_sqlite(
                &db_path,
                &backup_query,
                // Run in unsafe mode to allow backup process to
                // attach to destination database.
                true,
            )?;
            if !result.rows.is_empty() {
                return Err(AybError::SnapshotError {
                    message: format!("Unexpected snapshot result: {:?}", result),
                });
            }
            let result = query_sqlite(&snapshot_path, "PRAGMA integrity_check;", false)?;
            if result.fields.len() != 1
                || result.rows.len() != 1
                || result.rows[0][0] != Some("ok".to_string())
            {
                return Err(AybError::SnapshotError {
                    message: format!("Snapshot failed integrity check: {:?}", result),
                });
            }
            let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
            snapshot_storage
                .put(
                    &entity_slug,
                    &database_slug,
                    &Snapshot {
                        pre_snapshot_hash: "notimplemented".to_string(),
                        snapshot_hash: "notimplemented".to_string(),
                        snapshot_type: SnapshotType::Automatic as i16,
                    },
                    &snapshot_path,
                )
                .await?;
            let recent_snapshot = snapshot_storage
                .list_snapshots(&entity_slug, &database_slug)
                .await?
                .pop();
            println!("Storage: {:?}", recent_snapshot);
            println!("Completed snapshot");

            // Clean up after uploading snapshot.
            fs::remove_file(&snapshot_path).ok();
        }
        Err(err) => match err {
            AybError::RecordNotFound { record_type, .. } if record_type == "database" => {
                println!("Not a known database {}/{}", entity_slug, database_slug);
            }
            _ => {
                return Err(err);
            }
        },
    }
    Ok(())
}
