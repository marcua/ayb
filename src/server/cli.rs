use crate::ayb_db::db_interfaces::{connect_to_ayb_db, detect_ayb_db_type, AybDbType};
use crate::client::cli::{entity_database_parser, OutputFormat};
use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::http::structs::EntityDatabasePath;
use crate::server::config::{read_config, AybConfig};
use crate::server::snapshots::storage::SnapshotStorage;
use clap::builder::ValueParser;
use clap::{arg, value_parser, ArgMatches, Command};
use std::fs;
use std::path::{Path, PathBuf};

pub fn server_commands() -> Command {
    Command::new("server")
        .about("Run an HTTP server or perform server-side operations")
        .long_about(
            "Run an HTTP server or perform server-side operations. Configuration can be provided via:\n\
                    1. TOML file (--config flag, optional if all config in env vars)\n\
                    2. Environment variables with AYB__ prefix (use __ for all separators)\n\
                    Examples: AYB__HOST, AYB__PORT, AYB__AUTHENTICATION__FERNET_KEY",
        )
        .arg(
            arg!(--config <FILE> "Path to the server's configuration file (optional if using env vars)")
                .value_parser(value_parser!(PathBuf))
                .env("AYB_SERVER_CONFIG_FILE")
                .default_value("./ayb.toml"),
        )
        .subcommand(
            Command::new("list_snapshots")
                .about("List snapshots/backups of a database (server-side)")
                .arg(
                    arg!(<database> "The database for which to list snapshots (e.g., entity/database.sqlite or __ayb__/ayb for the metadata database)")
                        .value_parser(ValueParser::new(entity_database_parser))
                        .required(true),
                )
                .arg(
                    arg!(--format <type> "The format in which to output the result")
                        .value_parser(value_parser!(OutputFormat))
                        .default_value(OutputFormat::Table.to_str())
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("restore_snapshot")
                .about("Restore a database to a particular snapshot/backup (server-side)")
                .arg(
                    arg!(<database> "The database for which to restore a snapshot (e.g., entity/database.sqlite or __ayb__/ayb for the metadata database)")
                        .value_parser(ValueParser::new(entity_database_parser))
                        .required(true),
                )
                .arg(arg!(<snapshot_id> "The id of the snapshot to restore").required(true)),
        )
}

pub async fn execute_server_command(matches: &ArgMatches, run_server: bool) -> std::io::Result<()> {
    let config_path = matches
        .get_one::<PathBuf>("config")
        .expect("config has a default value");

    if run_server {
        // Import and call run_server from server_runner
        crate::server::server_runner::run_server(config_path).await
    } else if let Some(matches) = matches.subcommand_matches("list_snapshots") {
        list_snapshots_command(config_path, matches).await
    } else if let Some(matches) = matches.subcommand_matches("restore_snapshot") {
        restore_snapshot_command(config_path, matches).await
    } else {
        println!("No subcommand provided. Run with --help for options.");
        Ok(())
    }
}

async fn list_snapshots_command(config_path: &Path, matches: &ArgMatches) -> std::io::Result<()> {
    if let (Some(entity_database), Some(format)) = (
        matches.get_one::<EntityDatabasePath>("database"),
        matches.get_one::<OutputFormat>("format"),
    ) {
        match list_snapshots_impl(
            config_path,
            &entity_database.entity,
            &entity_database.database,
        )
        .await
        {
            Ok(snapshots) => {
                if snapshots.is_empty() {
                    println!(
                        "No snapshots for {}/{}",
                        entity_database.entity, entity_database.database
                    );
                } else {
                    match format {
                        OutputFormat::Table => snapshots.generate_table()?,
                        OutputFormat::Csv => snapshots.generate_csv()?,
                    }
                }
                Ok(())
            }
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Missing required arguments");
        std::process::exit(1);
    }
}

async fn restore_snapshot_command(config_path: &Path, matches: &ArgMatches) -> std::io::Result<()> {
    if let (Some(entity_database), Some(snapshot_id)) = (
        matches.get_one::<EntityDatabasePath>("database"),
        matches.get_one::<String>("snapshot_id"),
    ) {
        match restore_snapshot_impl(
            config_path,
            &entity_database.entity,
            &entity_database.database,
            snapshot_id,
        )
        .await
        {
            Ok(()) => {
                println!(
                    "Successfully restored {}/{} to snapshot {}",
                    entity_database.entity, entity_database.database, snapshot_id
                );
                Ok(())
            }
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Missing required arguments");
        std::process::exit(1);
    }
}

/// List snapshots for a database (shared implementation)
async fn list_snapshots_impl(
    config_path: &Path,
    entity: &str,
    database: &str,
) -> Result<Vec<crate::server::snapshots::models::ListSnapshotResult>, AybError> {
    let config = read_config(config_path)?;

    if config.snapshots.is_none() {
        return Err(AybError::SnapshotError {
            message: "Snapshots are not configured in the server config".to_string(),
        });
    }

    let snapshot_config = config.snapshots.as_ref().unwrap();
    let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;

    snapshot_storage.list_snapshots(entity, database).await
}

/// Restore a snapshot for a database (shared implementation)
async fn restore_snapshot_impl(
    config_path: &Path,
    entity: &str,
    database: &str,
    snapshot_id: &str,
) -> Result<(), AybError> {
    let config = read_config(config_path)?;

    if config.snapshots.is_none() {
        return Err(AybError::SnapshotError {
            message: "Snapshots are not configured in the server config".to_string(),
        });
    }

    let snapshot_config = config.snapshots.as_ref().unwrap();
    let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;

    // Special handling for ayb_db metadata database
    if entity == "__ayb__" && database == "ayb" {
        restore_ayb_db_snapshot(&config, &snapshot_storage, snapshot_id).await
    } else {
        // Restore regular hosted database
        restore_hosted_db_snapshot(&config, &snapshot_storage, entity, database, snapshot_id).await
    }
}

/// Restore the ayb_db metadata database from a snapshot
async fn restore_ayb_db_snapshot(
    config: &AybConfig,
    snapshot_storage: &SnapshotStorage,
    snapshot_id: &str,
) -> Result<(), AybError> {
    // Verify that ayb_db is SQLite
    if detect_ayb_db_type(&config.database_url)? != AybDbType::Sqlite {
        return Err(AybError::SnapshotError {
            message: "Only SQLite ayb_db can be restored via snapshots".to_string(),
        });
    }

    // Extract the file path from the database_url
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

    // Create a backup of the current ayb_db
    let backup_path = ayb_db_path.with_extension("sqlite.backup");
    if ayb_db_path.exists() {
        fs::copy(&ayb_db_path, &backup_path).map_err(|err| AybError::SnapshotError {
            message: format!(
                "Failed to create backup of ayb_db at {}: {}",
                backup_path.display(),
                err
            ),
        })?;
        println!("Created backup at {}", backup_path.display());
    }

    // Create a temporary directory for the snapshot
    let temp_dir = PathBuf::from("/tmp/ayb_restore");
    fs::create_dir_all(&temp_dir)?;

    // Retrieve the snapshot
    snapshot_storage
        .retrieve_snapshot("__ayb__", "ayb", snapshot_id, &temp_dir)
        .await?;

    // Move the snapshot to replace the current ayb_db
    let mut snapshot_path = temp_dir.clone();
    snapshot_path.push("ayb");
    fs::rename(&snapshot_path, &ayb_db_path).map_err(|err| AybError::SnapshotError {
        message: format!(
            "Failed to replace ayb_db with snapshot: {}. Restore from backup at {}",
            err,
            backup_path.display()
        ),
    })?;

    // Clean up temp directory
    fs::remove_dir_all(&temp_dir).ok();

    println!(
        "Successfully restored ayb_db. Original backed up to {}",
        backup_path.display()
    );
    println!("IMPORTANT: You must restart the server for changes to take effect!");

    Ok(())
}

/// Restore a hosted database from a snapshot
async fn restore_hosted_db_snapshot(
    config: &AybConfig,
    _snapshot_storage: &SnapshotStorage,
    entity: &str,
    database: &str,
    snapshot_id: &str,
) -> Result<(), AybError> {
    // Connect to ayb_db to verify the database exists
    let ayb_db = connect_to_ayb_db(config.database_url.clone()).await?;
    let _db = ayb_db.get_database(entity, database).await?;

    // For hosted databases, we need to use the daemon registry and proper paths
    // This is complex and requires the server to be running, so we provide guidance instead
    Err(AybError::SnapshotError {
        message: format!(
            "Restoring hosted databases via server CLI is not yet fully implemented.\n\
             To restore {}/{}, you can:\n\
             1. Use the API: ayb client restore_snapshot {}/{} {}\n\
             2. Or use the web interface to restore the snapshot\n\n\
             Server-side restore is only fully supported for __ayb__/ayb (the metadata database).",
            entity, database, entity, database, snapshot_id
        ),
    })
}
