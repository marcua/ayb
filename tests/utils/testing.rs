use assert_cmd::prelude::*;
use ayb::error::AybError;
use ayb::server::config::read_config;
use ayb::server::snapshots::storage::SnapshotStorage;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Once;

// ayb_cmd!("value1", value2; {
//     "ENV_VAR" => env_value
// })
macro_rules! ayb_cmd {
    ($($value:expr),+; { $($env_left:literal => $env_right:expr),* $(,)? }) => {
        Command::cargo_bin("ayb")?
                .args([$($value,)*])
                $(.env($env_left, $env_right))*
    }
}

pub struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all("/tmp/ayb/e2e") {
            assert_eq!(format!("{err}"), "No such file or directory (os error 2)")
        }
    }
}

pub fn generate_test_config(test_type: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Map test_type to port and slug
    let (port, slug) = match test_type {
        "postgres" => (5433, "postgres"),
        "sqlite" => (5434, "sqlite"),
        "browser_sqlite" => (5435, "browser_sqlite"),
        _ => return Err(format!("Unknown test_type: {}", test_type).into()),
    };

    let config_path = format!("tests/test-server-config-{slug}.toml");

    // Determine database configuration based on slug
    let (database_url, is_postgres) = if slug.contains("postgres") {
        (
            "postgresql://postgres_user:test@localhost:5432/test_db".to_string(),
            true,
        )
    } else {
        (format!("sqlite://tests/ayb_data_{slug}/ayb.sqlite"), false)
    };

    // Set S3 configuration based on slug
    let (s3_endpoint, s3_access_key, s3_secret_key, max_snapshots) = if slug.contains("browser") {
        ("http://localhost:4566", "test", "test", 3)
    } else {
        ("http://localhost:9000", "minioadmin", "minioadmin", 6)
    };

    let e2e_testing_line = if slug.contains("browser") {
        "e2e_testing = true\n"
    } else {
        ""
    };

    let config_content = format!(
        r#"host = "0.0.0.0"
port = {port}
database_url = "{database_url}"
data_path = "./tests/ayb_data_{slug}"
{e2e_testing_line}
[web]
hosting_method = "Local"

[email.file]
path = "tests/ayb_data_{slug}/emails.jsonl"

[authentication]
fernet_key = "y3UdMqGh6si7pvQb8wsuW3ryiJcacp0H1QoHUPfsjb0="
token_expiration_seconds = 3600

[cors]
origin = "*"

[isolation]
nsjail_path = "tests/nsjail"

[snapshots]
sqlite_method = "Vacuum"
access_key_id = "{s3_access_key}"
secret_access_key = "{s3_secret_key}"
bucket = "bucket"
path_prefix = "{path_prefix}"
endpoint_url = "{s3_endpoint}"
force_path_style = true

[snapshots.automation]
interval = "2s"
max_snapshots = {max_snapshots}
"#,
        port = port,
        database_url = database_url,
        slug = slug,
        e2e_testing_line = e2e_testing_line,
        s3_access_key = s3_access_key,
        s3_secret_key = s3_secret_key,
        path_prefix = if is_postgres { "postgres" } else { "sqlite" },
        s3_endpoint = s3_endpoint,
        max_snapshots = max_snapshots
    );

    // Write the configuration to file
    let mut file = std::fs::File::create(&config_path)?;
    file.write_all(config_content.as_bytes())?;

    Ok(config_path)
}

pub fn reset_test_environment(test_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = format!("./tests/ayb_data_{}", test_type);

    match test_type {
        "postgres" => {
            // Remove data directory
            if std::path::Path::new(&data_dir).exists() {
                std::fs::remove_dir_all(&data_dir)?;
            }

            // Drop and recreate PostgreSQL database
            let mut drop_cmd = Command::new("dropdb");
            drop_cmd
                .env("PGHOST", "localhost")
                .env("PGUSER", "postgres_user")
                .env("PGPASSWORD", "test")
                .arg("test_db");

            // Ignore error if database doesn't exist
            let _ = drop_cmd.output();

            let mut create_cmd = Command::new("createdb");
            create_cmd
                .env("PGHOST", "localhost")
                .env("PGUSER", "postgres_user")
                .env("PGPASSWORD", "test")
                .arg("test_db");

            let output = create_cmd.output()?;
            if !output.status.success() {
                return Err(format!(
                    "Failed to create PostgreSQL database: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        "sqlite" | "browser_sqlite" => {
            // Remove data directory
            if std::path::Path::new(&data_dir).exists() {
                std::fs::remove_dir_all(&data_dir)?;
            }
        }
        _ => return Err(format!("Unknown test_type: {}", test_type).into()),
    }

    Ok(())
}

static MINIO_INIT: Once = Once::new();

pub fn ensure_minio_running() -> Result<(), Box<dyn std::error::Error>> {
    MINIO_INIT.call_once(|| {
        if let Err(e) = setup_minio() {
            eprintln!("Failed to setup MinIO: {}", e);
            std::process::exit(1);
        }
    });
    Ok(())
}

fn setup_minio() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting MinIO (one-time setup)...");

    let output = Command::new("tests/run_minio.sh").output()?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    } else {
        eprintln!("MinIO setup failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        Err("Failed to run MinIO setup script".into())
    }
}

pub struct AybServer(Child);
impl AybServer {
    pub fn run(test_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = generate_test_config(test_type)?;

        Ok(Self(
            ayb_cmd!("server", "--config", &config_path; {
                "RUST_LOG" => "actix_web=debug",
                "RUST_BACKTRACE" => "1"
            })
            .spawn()?,
        ))
    }
}

impl Drop for AybServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

pub async fn snapshot_storage(test_type: &str) -> Result<SnapshotStorage, AybError> {
    let config_path = generate_test_config(test_type).map_err(|e| AybError::Other {
        message: e.to_string(),
    })?;
    let config = read_config(&PathBuf::from(config_path))?;
    SnapshotStorage::new(&config.snapshots.unwrap()).await
}
