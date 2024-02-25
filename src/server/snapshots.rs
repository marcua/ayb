use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedDatabase;
use crate::error::AybError;
use crate::hosted_db::paths::{
    database_parent_path, database_path, database_snapshot_path, pathbuf_to_file_name,
    pathbuf_to_parent,
};
use crate::hosted_db::sqlite::query_sqlite;
use crate::server::config::{AybConfig, AybConfigSnapshots, SqliteSnapshotMethod};
use go_parse_duration::parse_duration;
use s3::bucket::Bucket;
use s3::creds::Credentials;
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
                        create_snapshots(&config.clone(), &ayb_db.clone()).await;
                    })
                })?)
                .await?;
            scheduler.shutdown_on_ctrl_c();

            scheduler.start().await?;
        }
    }
    Ok(())
}

async fn create_snapshots(config: &AybConfig, ayb_db: &Box<dyn AybDb>) {
    // Walk the data path for entity slugs, database slugs
    for entry in WalkDir::new(database_parent_path(&config.data_path).unwrap())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir())
    {
        let path = entry.path();
        // For any database, get its hash and look that Snapshot up in the DB (or S3...decide)
        if let Some(err) = snapshot_database(&config, &ayb_db, &path).await.err() {
            eprintln!("Unable to snapshot database {}: {}", path.display(), err);
        }
    }
}

pub async fn snapshot_database(
    config: &AybConfig,
    ayb_db: &Box<dyn AybDb>,
    path: &Path,
) -> Result<(), AybError> {
    println!("Trying to back up {}", path.display());
    let entity_slug = pathbuf_to_file_name(&pathbuf_to_parent(&pathbuf_to_parent(path)?)?)?;
    let database_slug = pathbuf_to_file_name(&path)?;
    if let None = config.snapshots {
        return Err(AybError::SnapshotError {
            message: "No snapshot config found".to_string(),
        });
    }
    let snapshot_config = config.snapshots.as_ref().unwrap();

    match ayb_db.get_database(&entity_slug, &database_slug).await {
        Ok(db) => {
            println!("Hashing {} {}", entity_slug, database_slug);
            // TODO(marcua): Implement hashing. `.sha3sum --schema` is
            // only available at the SQLite command line since it's a
            // dot command.
            let db_path = database_path(&entity_slug, &database_slug, &config.data_path, false)?;
            // TODO(marcua): Do better than "temporary"
            // by creating a tmpdir.
            let mut snapshot_path = database_snapshot_path(
                &entity_slug,
                &database_slug,
                "temporary",
                &config.data_path,
            )?;
            snapshot_path.push(database_slug);
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
            println!("Running {}", backup_query);
            let result = query_sqlite(
                &db_path,
                &backup_query,
                // Run in unsafe mode to allow backup process to
                // attach to destination database.
                true,
            )?;
            if result.rows.len() != 0 {
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
            // TODO(marcua)
            // - Get hash (get fs::metadata of each file in the dir, call `modified()` on result, sort the times so it's stable, shasum those together).
            // - Upload to S3-like storage
            // - Clean up: Initialize a HostedDb that has a SQLite / DuckDB implementation. Push query/backup logic into that. Consider doing this on an InstantiatedDatabase directly.
            println!("Completed snapshot");
        }
        Err(err) => match err {
            AybError::RecordNotFound { record_type, .. } if record_type == "database" => {
                println!("Not a known database {}/{}", entity_slug, database_slug);
            }
            _ => {
                return Err(AybError::from(err));
            }
        },
    }
    Ok(())
}

pub struct SnapshotStorage {
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
    path_prefix: String,
    endpoint: Option<String>,
    region: Option<String>,
    force_path_style: Option<bool>,
}

impl SnapshotStorage {
    fn create(config: &AybConfigSnapshots) -> SnapshotStorage {
        let credentials = Credentials::default(...)?;
        SnapshotStorage {
            bucket: Bucket::new(config.bucket, config.region.or("us-east-1".to_string), credentials)?,
            path_prefix: config.path_prefix,
        }
        let bucket_name = "rust-s3-test";
        let region = "us-east-1".parse()?;

        let bucket = 
    }
    fn db_path(self, entity_slug: &str, database_slug: &str, final_path: &str) -> String {
        format!("{}/{}/{}/{}", self.path_prefix, entity_slug, database_slug, final_path)
    }
    
    pub fn list_snapshots(self, entity_slug: &str, database_slug: &str) {
        self.path_to_db(entity_slug, database_slug, "")
    }
}
