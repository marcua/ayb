use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::hosted_db::paths::{
    current_database_path, database_parent_path, database_snapshot_path, pathbuf_to_file_name,
    pathbuf_to_parent,
};
use crate::hosted_db::sqlite::query_sqlite;
use crate::hosted_db::QueryMode;
use crate::server::config::{AybConfig, SqliteSnapshotMethod};
use crate::server::snapshots::hashes::hash_db_directory;
use crate::server::snapshots::models::{Snapshot, SnapshotType};
use crate::server::snapshots::storage::SnapshotStorage;
use go_parse_duration::parse_duration;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn schedule_periodic_snapshots(
    config: AybConfig,
    ayb_db: Box<dyn AybDb>,
) -> Result<(), AybError> {
    if let Some(ref snapshot_config) = config.snapshots {
        if let Some(ref automation_config) = snapshot_config.automation {
            let scheduler = JobScheduler::new().await?;
            let duration = Duration::from_nanos(
                parse_duration(&automation_config.interval)?
                    .try_into()
                    .map_err(|err| AybError::SnapshotError {
                        message: format!(
                            "Unable to turn snapshot interval into a duration: {err:?}"
                        ),
                    })?,
            );
            // Since jobs are scheduled to run on an interval, it's
            // possible that if it takes a while to snapshot
            // databases, two snapshot jobs will run at the same
            // time. To avoid asynchrony-related issues, we skip a
            // snapshot run if a previous one is running.
            let is_running = Arc::new(Mutex::new(false));
            scheduler
                .add(Job::new_repeated_async(duration, move |_, _| {
                    let is_running = Arc::clone(&is_running);
                    let config = config.clone();
                    let ayb_db = ayb_db.clone();
                    Box::pin(async move {
                        let mut guard = is_running.lock().await;
                        if *guard {
                            println!("Previous snapshot logic running, will skip this round...");
                            return;
                        }
                        // Mark the job as running
                        *guard = true;
                        if let Some(err) = create_snapshots(&config.clone(), &ayb_db.clone())
                            .await
                            .err()
                        {
                            eprintln!("Unable to walk database directory for snapshots: {err}");
                        }
                        *guard = false;
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
    let entity_paths =
        fs::read_dir(database_parent_path(&config.data_path, true).unwrap())?.map(|entry| {
            let entry_path = entry?.path();
            let entity = pathbuf_to_file_name(&entry_path)?;
            if entry_path.is_dir() {
                Ok((entity, entry_path))
            } else {
                Err(AybError::SnapshotError {
                    message: format!(
                        "Unexpected file where entity directory expected: {}",
                        entry_path.display()
                    ),
                })
            }
        });
    for entity_details in entity_paths {
        let (entity, entity_path) = entity_details?;
        for entry in fs::read_dir(entity_path)? {
            let entry_path = entry?.path();
            let database = pathbuf_to_file_name(&entry_path)?;
            if entry_path.is_dir() {
                if let Some(err) = snapshot_database(config, ayb_db, &entity, &database)
                    .await
                    .err()
                {
                    eprintln!("Unable to snapshot database {entity}/{database}: {err}");
                }
            } else {
                return Err(AybError::SnapshotError {
                    message: format!(
                        "Unexpected file where database directory expected: {}",
                        entry_path.display()
                    ),
                });
            }
        }
    }

    // If ayb_db is SQLite, snapshot it as well
    if config.database_url.starts_with("sqlite") {
        println!("Trying to back up __ayb__/ayb (ayb_db itself)");
        // Extract the file path from the sqlite:// URL
        let db_file_path =
            config
                .database_url
                .strip_prefix("sqlite://")
                .ok_or(AybError::SnapshotError {
                    message: format!(
                        "Unable to parse SQLite path from database_url: {}",
                        config.database_url
                    ),
                })?;
        let ayb_db_path = PathBuf::from(db_file_path);

        // Canonicalize the path to get the absolute path
        let ayb_db_path =
            fs::canonicalize(&ayb_db_path).map_err(|err| AybError::SnapshotError {
                message: format!(
                    "Unable to canonicalize ayb_db path {}: {}",
                    ayb_db_path.display(),
                    err
                ),
            })?;

        if let Some(err) = perform_snapshot(config, "__ayb__", "ayb", &ayb_db_path)
            .await
            .err()
        {
            eprintln!("Unable to snapshot ayb_db: {err}");
        }
    }

    Ok(())
}

/// Core snapshotting logic that performs the actual snapshot, integrity check, and upload.
/// This is called both for regular hosted databases and for the ayb_db itself.
async fn perform_snapshot(
    config: &AybConfig,
    entity_slug: &str,
    database_slug: &str,
    db_path: &PathBuf,
) -> Result<(), AybError> {
    if config.snapshots.is_none() {
        return Err(AybError::SnapshotError {
            message: "No snapshot config found".to_string(),
        });
    }
    let snapshot_config = config.snapshots.as_ref().unwrap();

    let mut snapshot_path = database_snapshot_path(entity_slug, database_slug, &config.data_path)?;
    let snapshot_directory = snapshot_path.clone();
    snapshot_path.push(database_slug);
    // Try to remove the file if it already exists, but don't fail if it doesn't.
    fs::remove_file(&snapshot_path).ok();
    let backup_query = match snapshot_config.sqlite_method {
        // TODO(marcua): Figure out dot commands to make .backup work
        SqliteSnapshotMethod::Backup => {
            return Err(AybError::SnapshotError {
                message: "Backup requires dot commands, which are not yet supported".to_string(),
            })
        }
        SqliteSnapshotMethod::Vacuum => {
            format!("VACUUM INTO \"{}\"", snapshot_path.display())
        }
    };
    let result = query_sqlite(
        db_path,
        &backup_query,
        // Run in unsafe mode to allow backup process to
        // attach to destination database.
        true,
        QueryMode::ReadOnly,
    )?;
    if !result.rows.is_empty() {
        return Err(AybError::SnapshotError {
            message: format!("Unexpected snapshot result: {result:?}"),
        });
    }
    let result = query_sqlite(
        &snapshot_path,
        "PRAGMA integrity_check;",
        false,
        QueryMode::ReadOnly,
    )?;
    if result.fields.len() != 1
        || result.rows.len() != 1
        || result.rows[0][0] != Some("ok".to_string())
    {
        return Err(AybError::SnapshotError {
            message: format!("Snapshot failed integrity check: {result:?}"),
        });
    }

    let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
    let existing_snapshots = snapshot_storage
        .list_snapshots(entity_slug, database_slug)
        .await?;
    let num_existing_snapshots = existing_snapshots.len();
    let snapshot_hash = hash_db_directory(&snapshot_directory)?;
    let mut should_upload_snapshot = true;
    for snapshot in &existing_snapshots {
        if snapshot.snapshot_id == snapshot_hash {
            println!("Snapshot with hash {snapshot_hash} already exists, not uploading again.");
            should_upload_snapshot = false;
            break;
        }
    }
    if should_upload_snapshot {
        println!("Uploading new snapshot with hash {snapshot_hash}.");
        snapshot_storage
            .put(
                entity_slug,
                database_slug,
                &Snapshot {
                    snapshot_id: snapshot_hash,
                    snapshot_type: SnapshotType::Automatic as i16,
                },
                &snapshot_path,
            )
            .await?;

        // If adding this snapshot resulted in more than the
        // maximum snapshots we are allowed, prune old ones.
        let max_snapshots: usize = snapshot_config
            .automation
            .as_ref()
            .unwrap()
            .max_snapshots
            .into();
        let prune_snapshots = (num_existing_snapshots + 1).checked_sub(max_snapshots);
        if let Some(prune_snapshots) = prune_snapshots {
            println!("Pruning {prune_snapshots} oldest snapshots");
            let mut ids_to_prune: Vec<String> = vec![];
            for snapshot_index in 0..prune_snapshots {
                ids_to_prune.push(
                    existing_snapshots[existing_snapshots.len() - snapshot_index - 1]
                        .snapshot_id
                        .clone(),
                )
            }

            snapshot_storage
                .delete_snapshots(entity_slug, database_slug, &ids_to_prune)
                .await?;
        }
    }

    // Clean up after uploading snapshot.
    fs::remove_dir_all(pathbuf_to_parent(&snapshot_path)?)?;
    Ok(())
}

#[allow(clippy::borrowed_box)]
pub async fn snapshot_database(
    config: &AybConfig,
    ayb_db: &Box<dyn AybDb>,
    entity_slug: &str,
    database_slug: &str,
) -> Result<(), AybError> {
    println!("Trying to back up {entity_slug}/{database_slug}");
    match ayb_db.get_database(entity_slug, database_slug).await {
        Ok(_db) => {
            let db_path = current_database_path(entity_slug, database_slug, &config.data_path)?;
            perform_snapshot(config, entity_slug, database_slug, &db_path).await?;
        }
        Err(err) => match err {
            AybError::RecordNotFound { record_type, .. } if record_type == "database" => {
                println!("Not a known database {entity_slug}/{database_slug}");
            }
            _ => {
                return Err(err);
            }
        },
    }
    Ok(())
}
