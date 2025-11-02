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
use crate::hosted_db::paths::{pathbuf_to_file_name, pathbuf_to_parent};
use std::env::current_exe;
use std::fs::canonicalize;
use std::path::{Path, PathBuf};

/// Build command for running daemon with nsjail isolation
pub fn build_nsjail_command(
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

    // Set resource limits for the process. In the future, we will
    // allow entities to control the resources they dedicate to
    // different databases/queries.
    cmd.args(["--mount", "none:/tmp:tmpfs:size=100000000"]) // ~95 MB tmpfs
        .args(["--max_cpus", "1"]) // One CPU
        .args(["--rlimit_as", "64"]) // 64 MB memory limit
        .args(["--time_limit", "0"]) // No time limit for daemon
        .args(["--rlimit_fsize", "75"]) // 75 MB file size limit
        .args(["--rlimit_nofile", "10"]) // 10 files maximum
        .args(["--rlimit_nproc", "2"]); // 2 processes maximum

    // Map the database file
    let absolute_db_path = canonicalize(db_path)?;
    let db_file_name = pathbuf_to_file_name(&absolute_db_path)?;
    let tmp_db_path = Path::new("/tmp").join(db_file_name);
    let db_file_mapping = format!("{}:{}", absolute_db_path.display(), tmp_db_path.display());
    cmd.args(["--bindmount", &db_file_mapping]);

    // Map the query_daemon binary
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");
    cmd.args([
        "--bindmount_ro",
        &format!("{}:/tmp/ayb_query_daemon", query_daemon_path.display()),
    ]);

    // Run the daemon
    cmd.arg("--").arg("/tmp/ayb_query_daemon").arg(tmp_db_path);

    Ok(cmd)
}

/// Build command for running daemon without isolation
pub fn build_direct_command(db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");

    let mut cmd = tokio::process::Command::new(&query_daemon_path);
    cmd.arg(db_path);

    Ok(cmd)
}
