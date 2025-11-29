//! Sandbox command builders for database isolation.
//!
//! This module provides command builders for running the query daemon
//! with native isolation (Landlock, rlimits, cgroups).

use crate::error::AybError;
use crate::hosted_db::paths::pathbuf_to_parent;
use std::env::current_exe;
use std::path::PathBuf;

/// Build command for running daemon with native isolation (Landlock, rlimits, cgroups).
/// This is the only method available - isolation is always enabled.
pub fn build_isolated_command(db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");

    let mut cmd = tokio::process::Command::new(&query_daemon_path);
    cmd.arg(db_path);
    cmd.arg("--isolate"); // Enable native isolation

    Ok(cmd)
}
