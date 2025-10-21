use crate::error::AybError;
use crate::hosted_db::paths::{pathbuf_to_file_name, pathbuf_to_parent};
use crate::hosted_db::QueryMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::current_exe;
use std::fs::canonicalize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdin};

#[derive(Serialize, Deserialize, Debug)]
struct QueryRequest {
    query: String,
    query_mode: i16,
}

/// Handle to a running daemon process for a specific database
pub struct DaemonHandle {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl DaemonHandle {
    /// Send a query to the daemon and read the response
    pub async fn execute_query(
        &mut self,
        query: &str,
        query_mode: QueryMode,
    ) -> Result<String, AybError> {
        // Serialize and send the request
        let request = QueryRequest {
            query: query.to_string(),
            query_mode: query_mode as i16,
        };
        let request_json = serde_json::to_string(&request)?;

        // Write to daemon's stdin
        use tokio::io::AsyncWriteExt;
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response from daemon's stdout
        use tokio::io::AsyncBufReadExt;
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        Ok(response_line)
    }
}

/// Registry of daemon processes, one per database path
pub struct DaemonRegistry {
    daemons: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<DaemonHandle>>>>>,
}

impl DaemonRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            daemons: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a daemon for the given database path
    /// Returns an Arc<Mutex<DaemonHandle>> that can be used across threads
    pub async fn get_or_create_daemon(
        &self,
        db_path: &PathBuf,
        nsjail_path: Option<&Path>,
    ) -> Result<Arc<Mutex<DaemonHandle>>, AybError> {
        // Canonicalize the path to ensure consistency
        let canonical_path = canonicalize(db_path)?;

        // First, try to get an existing daemon
        {
            let daemons = self.daemons.lock().unwrap();
            if let Some(daemon) = daemons.get(&canonical_path) {
                return Ok(daemon.clone());
            }
        }

        // No existing daemon, need to create one
        // Spawn the daemon process
        let daemon_handle = self.spawn_daemon(&canonical_path, nsjail_path).await?;
        let daemon_arc = Arc::new(Mutex::new(daemon_handle));

        // Insert into registry (check again in case another thread created it)
        let mut daemons = self.daemons.lock().unwrap();
        if let Some(existing) = daemons.get(&canonical_path) {
            // Another thread beat us to it, use theirs
            Ok(existing.clone())
        } else {
            // We're first, insert ours
            daemons.insert(canonical_path, daemon_arc.clone());
            Ok(daemon_arc)
        }
    }

    /// Spawn a new daemon process for the given database
    async fn spawn_daemon(
        &self,
        db_path: &PathBuf,
        nsjail_path: Option<&Path>,
    ) -> Result<DaemonHandle, AybError> {
        let mut cmd = if let Some(nsjail) = nsjail_path {
            // Spawn with nsjail isolation
            self.build_nsjail_command(nsjail, db_path)?
        } else {
            // Spawn without isolation
            self.build_direct_command(db_path)?
        };

        // Spawn the process with piped stdin/stdout
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // We don't use stderr in daemon mode
            .spawn()?;

        let stdin = child.stdin.take().ok_or(AybError::Other {
            message: "Failed to get daemon stdin".to_string(),
        })?;

        let stdout = child.stdout.take().ok_or(AybError::Other {
            message: "Failed to get daemon stdout".to_string(),
        })?;

        Ok(DaemonHandle {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Build command for running daemon with nsjail isolation
    fn build_nsjail_command(
        &self,
        nsjail: &Path,
        db_path: &PathBuf,
    ) -> Result<tokio::process::Command, AybError> {
        let mut cmd = tokio::process::Command::new(nsjail);

        cmd.arg("--really_quiet") // log fatal messages only
            .arg("--iface_no_lo")
            .args(["--mode", "o"]) // run once
            .args(["--hostname", "ayb"])
            .args(["--bindmount_ro", "/lib:/lib"])
            .args(["--bindmount_ro", "/lib64:/lib64"])
            .args(["--bindmount_ro", "/usr:/usr"]);

        // Set resource limits
        cmd.args(["--mount", "none:/tmp:tmpfs:size=100000000"]) // ~95 MB tmpfs
            .args(["--max_cpus", "1"])
            .args(["--rlimit_as", "64"]) // 64 MB memory limit
            .args(["--time_limit", "0"]) // No time limit for daemon
            .args(["--rlimit_fsize", "75"])
            .args(["--rlimit_nofile", "10"])
            .args(["--rlimit_nproc", "2"]);

        // Map the database file
        let absolute_db_path = canonicalize(db_path)?;
        let db_file_name = pathbuf_to_file_name(&absolute_db_path)?;
        let tmp_db_path = Path::new("/tmp").join(db_file_name);
        let db_file_mapping = format!("{}:{}", absolute_db_path.display(), tmp_db_path.display());
        cmd.args(["--bindmount", &db_file_mapping]);

        // Map the isolated_runner binary
        let ayb_path = current_exe()?;
        let isolated_runner_path = pathbuf_to_parent(&ayb_path)?.join("ayb_isolated_runner");
        cmd.args([
            "--bindmount_ro",
            &format!(
                "{}:/tmp/ayb_isolated_runner",
                isolated_runner_path.display()
            ),
        ]);

        // Run the daemon
        cmd.arg("--")
            .arg("/tmp/ayb_isolated_runner")
            .arg("--daemon")
            .arg(tmp_db_path);

        Ok(cmd)
    }

    /// Build command for running daemon without isolation
    fn build_direct_command(&self, db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
        let ayb_path = current_exe()?;
        let isolated_runner_path = pathbuf_to_parent(&ayb_path)?.join("ayb_isolated_runner");

        let mut cmd = tokio::process::Command::new(&isolated_runner_path);
        cmd.arg("--daemon").arg(db_path);

        Ok(cmd)
    }

    /// Shutdown all running daemons
    pub async fn shutdown_all(&self) {
        let mut daemons = self.daemons.lock().unwrap();
        for (_path, daemon_arc) in daemons.drain() {
            // Try to get exclusive access to kill the daemon
            if let Ok(mut daemon) = daemon_arc.try_lock() {
                // Closing stdin will cause the daemon to exit gracefully
                drop(daemon.stdin);
                // The child process will be killed when DaemonHandle is dropped
            }
        }
    }
}

impl Clone for DaemonRegistry {
    fn clone(&self) -> Self {
        Self {
            daemons: self.daemons.clone(),
        }
    }
}
