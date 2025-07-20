use assert_cmd::prelude::*;
use ayb::error::AybError;
use ayb::server::config::read_config;
use ayb::server::snapshots::storage::SnapshotStorage;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};

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

fn server_config_path(db_type: &str) -> String {
    format!("tests/test-server-config-{db_type}.toml")
}

fn browser_server_config_path(db_type: &str) -> String {
    format!("tests/test-server-config-browser-{}.toml", db_type)
}

pub struct AybServer(Child);
impl AybServer {
    pub fn run(db_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(
            ayb_cmd!("server", "--config", &server_config_path(db_type); {
                "RUST_LOG" => "actix_web=debug",
                "RUST_BACKTRACE" => "1"
            })
            .spawn()?,
        ))
    }

    pub fn run_browser(db_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(
            ayb_cmd!("server", "--config", &browser_server_config_path(db_type); {
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

pub async fn snapshot_storage(db_type: &str) -> Result<SnapshotStorage, AybError> {
    let config = read_config(&PathBuf::from(server_config_path(db_type)))?;
    SnapshotStorage::new(&config.snapshots.unwrap()).await
}
