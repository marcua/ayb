use crate::error::AybError;
use crate::hosted_db::sandbox::build_isolated_command;
use crate::hosted_db::{QueryMode, QueryResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct QueryRequest {
    query: String,
    query_mode: i16,
}

/// Handle to a running daemon process for a specific database
pub struct DaemonHandle {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl DaemonHandle {
    /// Send a query to the daemon and read the response
    pub async fn execute_query(
        &mut self,
        query: &str,
        query_mode: QueryMode,
    ) -> Result<String, AybError> {
        let stdin = self.stdin.as_mut().ok_or(AybError::Other {
            message: "Daemon stdin has been closed".to_string(),
        })?;

        // Serialize and send the request
        let request = QueryRequest {
            query: query.to_string(),
            query_mode: query_mode as i16,
        };
        let request_json = serde_json::to_string(&request)?;

        // Write to daemon's stdin
        use tokio::io::AsyncWriteExt;
        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response from daemon's stdout
        use tokio::io::AsyncBufReadExt;
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        Ok(response_line)
    }

    /// Shut down the daemon by closing stdin and killing the process
    pub async fn shut_down(&mut self) {
        // Close stdin to signal daemon to exit gracefully
        self.stdin.take();
        // Kill the process if still running
        let _ = self.child.kill().await;
    }
}

/// Registry of daemon processes, one per database path
pub struct DaemonRegistry {
    daemons: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<DaemonHandle>>>>>,
}

impl Default for DaemonRegistry {
    fn default() -> Self {
        Self::new()
    }
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
    async fn get_or_create_daemon(
        &self,
        db_path: &PathBuf,
    ) -> Result<Arc<Mutex<DaemonHandle>>, AybError> {
        // Canonicalize the path to ensure consistency
        let canonical_path = canonicalize(db_path)?;

        // Lock for the entire check-and-create operation to avoid race condition
        // where multiple threads spawn daemon processes for the same database
        let mut daemons = self.daemons.lock().await;

        // Check if daemon already exists
        if let Some(daemon) = daemons.get(&canonical_path) {
            return Ok(daemon.clone());
        }

        // Spawn the daemon process while holding the lock
        let daemon_handle = self.spawn_daemon(&canonical_path).await?;
        let daemon_arc = Arc::new(Mutex::new(daemon_handle));

        // Insert into registry
        daemons.insert(canonical_path, daemon_arc.clone());
        Ok(daemon_arc)
    }

    /// Execute a query by getting/creating daemon, locking, and executing
    pub async fn execute_query(
        &self,
        db_path: &PathBuf,
        query: &str,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        let daemon_arc = self.get_or_create_daemon(db_path).await?;
        let mut daemon = daemon_arc.lock().await;
        let response = daemon.execute_query(query, query_mode).await?;
        parse_response(&response)
    }

    /// Spawn a new daemon process for the given database
    /// Always uses native isolation (Landlock, seccomp, rlimits)
    async fn spawn_daemon(&self, db_path: &PathBuf) -> Result<DaemonHandle, AybError> {
        let mut cmd = build_isolated_command(db_path)?;

        // Spawn the process with piped stdin/stdout
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().ok_or(AybError::Other {
            message: "Failed to get daemon stdin".to_string(),
        })?;

        let stdout = child.stdout.take().ok_or(AybError::Other {
            message: "Failed to get daemon stdout".to_string(),
        })?;

        Ok(DaemonHandle {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }

    /// Shut down a daemon for a specific database path
    pub async fn shut_down_daemon(&self, db_path: &PathBuf) -> Result<(), AybError> {
        let canonical_path = canonicalize(db_path)?;

        let mut daemons = self.daemons.lock().await;
        if let Some(daemon_arc) = daemons.remove(&canonical_path) {
            // Try to get exclusive access to shut down the daemon
            if let Ok(mut daemon) = daemon_arc.try_lock() {
                daemon.shut_down().await;
            }
        }
        Ok(())
    }

    /// Shut down all running daemons
    pub async fn shut_down_all(&self) {
        let mut daemons = self.daemons.lock().await;
        for (_path, daemon_arc) in daemons.drain() {
            // Try to get exclusive access to shut down the daemon
            if let Ok(mut daemon) = daemon_arc.try_lock() {
                daemon.shut_down().await;
            }
        }
    }
}

/// Parse a JSON response from daemon into QueryResult or AybError
fn parse_response(response: &str) -> Result<QueryResult, AybError> {
    // Try to parse as QueryResult first
    if let Ok(result) = serde_json::from_str::<QueryResult>(response) {
        return Ok(result);
    }

    // Try to parse as AybError
    if let Ok(error) = serde_json::from_str::<AybError>(response) {
        return Err(error);
    }

    // If neither worked, return a generic error
    Err(AybError::QueryError {
        message: format!("Invalid response: {response}"),
    })
}

impl Clone for DaemonRegistry {
    fn clone(&self) -> Self {
        Self {
            daemons: self.daemons.clone(),
        }
    }
}
