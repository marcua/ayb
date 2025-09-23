use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::server::config::{read_config, AybConfig};
use crate::server::snapshots::storage::SnapshotStorage;
use crate::server::utils::extract_sqlite_file_path;
use clap::{arg, value_parser, ArgMatches, Command};
use std::fs;
use std::path::{Path, PathBuf};

/// Create admin command structure
pub fn admin_commands() -> Command {
    Command::new("admin")
        .about("Admin commands for system management (local server access only)")
        .arg(
            arg!(--config <FILE> "Path to the server configuration file")
                .value_parser(value_parser!(PathBuf))
                .required(true),
        )
        .subcommand(
            Command::new("list_system_snapshots").about("List all system database snapshots"),
        )
        .subcommand(
            Command::new("restore_system_snapshot")
                .about("Restore system database from a snapshot")
                .arg(arg!(<snapshot_id> "The ID of the snapshot to restore").required(true)),
        )
}

/// Execute admin commands
pub async fn execute_admin_command(matches: &ArgMatches) -> Result<(), AybError> {
    let config_path = matches
        .get_one::<PathBuf>("config")
        .ok_or_else(|| AybError::Other {
            message: "Config file path is required for admin commands".to_string(),
        })?;

    let config = read_config(config_path).map_err(|e| AybError::Other {
        message: format!("Unable to read configuration file: {e}"),
    })?;

    if let Some(_matches) = matches.subcommand_matches("list_system_snapshots") {
        list_system_snapshots(&config).await?;
    } else if let Some(matches) = matches.subcommand_matches("restore_system_snapshot") {
        if let Some(snapshot_id) = matches.get_one::<String>("snapshot_id") {
            restore_system_snapshot(&config, snapshot_id).await?;
        }
    }

    Ok(())
}

/// List all system database snapshots
async fn list_system_snapshots(config: &AybConfig) -> Result<(), AybError> {
    if config.snapshots.is_none() {
        return Err(AybError::Other {
            message: "Snapshots are not configured".to_string(),
        });
    }

    let snapshot_config = config.snapshots.as_ref().unwrap();
    let storage = SnapshotStorage::new(snapshot_config).await?;

    // Use reserved entity/database identifiers for system snapshots
    let entity_slug = "__system__";
    let database_slug = "ayb.sqlite";

    let snapshots = storage.list_snapshots(entity_slug, database_slug).await?;

    if snapshots.is_empty() {
        println!("No system snapshots found");
    } else {
        println!("System Database Snapshots:");
        snapshots.generate_table().map_err(|e| AybError::Other {
            message: format!("Failed to generate table: {e}"),
        })?;
    }

    Ok(())
}

/// Restore system database from a snapshot
async fn restore_system_snapshot(config: &AybConfig, snapshot_id: &str) -> Result<(), AybError> {
    if config.snapshots.is_none() {
        return Err(AybError::Other {
            message: "Snapshots are not configured".to_string(),
        });
    }

    // Verify the database URL is SQLite
    if !config.database_url.starts_with("sqlite://") {
        return Err(AybError::Other {
            message: "System database restore is only supported for SQLite databases".to_string(),
        });
    }

    let snapshot_config = config.snapshots.as_ref().unwrap();
    let storage = SnapshotStorage::new(snapshot_config).await?;

    // Use reserved entity/database identifiers for system snapshots
    let entity_slug = "__system__";
    let database_slug = "ayb.sqlite";

    // Get the current system database file path
    let system_db_path = extract_sqlite_file_path(&config.database_url)?;

    // Verify the system database file exists
    if !system_db_path.exists() {
        return Err(AybError::Other {
            message: format!(
                "System database file does not exist: {}",
                system_db_path.display()
            ),
        });
    }

    println!("Restoring system database to snapshot {snapshot_id}...");

    // Create a temporary file for the restored snapshot
    let temp_db_path = system_db_path.with_extension("sqlite.tmp");

    // Download and restore the snapshot
    storage
        .retrieve_snapshot(
            entity_slug,
            database_slug,
            snapshot_id,
            temp_db_path.parent().unwrap(),
        )
        .await?;

    // The retrieve_snapshot creates a file named after the database_slug in the target directory
    let downloaded_snapshot = temp_db_path.parent().unwrap().join(database_slug);

    // Verify the downloaded snapshot exists
    if !downloaded_snapshot.exists() {
        return Err(AybError::Other {
            message: "Downloaded snapshot file not found".to_string(),
        });
    }

    // Atomically replace the system database file
    atomic_file_replace(&downloaded_snapshot, &system_db_path)?;

    println!("✓ System database restored successfully");
    println!("⚠️  WARNING: You must restart the ayb server for changes to take effect");
    println!("   The server needs to reload its database connections");

    Ok(())
}

/// Atomically replace a file by moving the new file over the old one
fn atomic_file_replace(source: &Path, target: &Path) -> Result<(), AybError> {
    // Create a backup of the original file
    let backup_path = target.with_extension("sqlite.backup");

    if target.exists() {
        fs::copy(target, &backup_path)?;
    }

    // Atomically move the new file to replace the old one
    match fs::rename(source, target) {
        Ok(()) => {
            // Success - remove backup
            if backup_path.exists() {
                fs::remove_file(&backup_path).ok(); // Don't fail on cleanup
            }
            Ok(())
        }
        Err(e) => {
            // Failure - restore from backup if it exists
            if backup_path.exists() {
                fs::rename(&backup_path, target).ok(); // Don't fail on restore attempt
            }
            Err(AybError::Other {
                message: format!("Failed to replace database file: {e}"),
            })
        }
    }
}
