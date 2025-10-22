/* Retrieved and modified from
  https://raw.githubusercontent.com/Defelo/sandkasten/83f629175d02ebc70fbb16b8b9e05663ea67ccc7/src/sandbox.rs
  On December 6, 2023.
  Original license:

    MIT License

    Copyright (c) 2023 Defelo

    Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::{QueryMode, QueryResult};
use std::path::{Path, PathBuf};

/// Execute a query using a persistent daemon process with nsjail isolation
pub async fn run_daemon_query_in_sandbox(
    daemon_registry: &DaemonRegistry,
    nsjail: &Path,
    db_path: &PathBuf,
    query: &str,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    // Get or create the daemon for this database
    let daemon_arc = daemon_registry
        .get_or_create_daemon(db_path, Some(nsjail))
        .await?;

    // Execute the query through the daemon
    let mut daemon = daemon_arc.lock().await;
    let response = daemon.execute_query(query, query_mode).await?;

    // Parse the response
    parse_daemon_response(&response)
}

/// Execute a query using a persistent daemon process without isolation
pub async fn run_daemon_query_without_sandbox(
    daemon_registry: &DaemonRegistry,
    db_path: &PathBuf,
    query: &str,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    // Get or create the daemon for this database
    let daemon_arc = daemon_registry.get_or_create_daemon(db_path, None).await?;

    // Execute the query through the daemon
    let mut daemon = daemon_arc.lock().await;
    let response = daemon.execute_query(query, query_mode).await?;

    // Parse the response
    parse_daemon_response(&response)
}

/// Parse a JSON response from the daemon
fn parse_daemon_response(response: &str) -> Result<QueryResult, AybError> {
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
        message: format!("Invalid response from daemon: {}", response),
    })
}
