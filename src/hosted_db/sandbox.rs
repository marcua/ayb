//! Sandbox command builders for database isolation.
//!
//! This module provides command builders for running the query daemon
//! with various isolation mechanisms.

use crate::error::AybError;
use crate::hosted_db::paths::pathbuf_to_parent;
use std::env::current_exe;
use std::path::PathBuf;

/// Build command for running daemon with native isolation (Landlock, seccomp, rlimits).
/// This is the preferred method on Linux as it works in Docker containers.
pub fn build_isolated_command(db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");

    let mut cmd = tokio::process::Command::new(&query_daemon_path);
    cmd.arg(db_path);
    cmd.arg("--isolate"); // Enable native isolation

    Ok(cmd)
}

/// Build command for running daemon without isolation.
/// Used for development or when isolation is explicitly disabled.
pub fn build_direct_command(db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");

    let mut cmd = tokio::process::Command::new(&query_daemon_path);
    cmd.arg(db_path);

    Ok(cmd)
}
