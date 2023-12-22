/* Retrieved and modified from
  https://raw.githubusercontent.com/Defelo/sandkasten/83f629175d02ebc70fbb16b8b9e05663ea67ccc7/src/sandbox.rs
  On December 6, 2023.
  Original license: MIT.
*/

use crate::error::AybError;
use std::fs::canonicalize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunResult {
    /// The exit code of the processes.
    pub status: i32,
    /// The stdout output the process produced.
    pub stdout: String,
    /// The stderr output the process produced.
    pub stderr: String,
}

pub async fn run_in_sandbox(
    nsjail: &Path,
    db_path: &PathBuf,
    query: &str,
) -> Result<RunResult, AybError> {
    let mut cmd = tokio::process::Command::new(nsjail);

    cmd.arg("--really_quiet") // log fatal messages only
        .arg("--iface_no_lo")
        .args(["--mode", "o"]) // run once
        .args(["--hostname", "ayb"])
        .args(["--bindmount_ro", "/lib:/lib"])
        .args(["--bindmount_ro", "/lib64:/lib64"])
        .args(["--bindmount_ro", "/usr:/usr"])
        .args(["--mount", "none:/tmp:tmpfs:size=100000000"]) // TODO(marcua): Restrict disk size more configurably?
        // TODO(marcua): Set resource limits more configurably?
        .args(["--max_cpus", "1"])
        .args(["--rlimit_as", "64"]) // in MB
        .args(["--time_limit", "10"]) // in seconds
        .args(["--rlimit_fsize", "75"]) // in MB
        .args(["--rlimit_nofile", "10"])
        .args(["--rlimit_nproc", "2"]);

    // Generate a /local/path/to/file:/tmp/file mapping.
    let absolute_db_path = canonicalize(db_path)?;
    let db_file_name = absolute_db_path
        .file_name()
        .ok_or(AybError {
            message: format!("Invalid DB path {}", absolute_db_path.display()),
        })?
        .to_str()
        .ok_or(AybError {
            message: format!("Invalid DB path {}", absolute_db_path.display()),
        })?;
    let tmp_db_path = Path::new("/tmp").join(db_file_name);
    let db_file_mapping = format!("{}:{}", absolute_db_path.display(), tmp_db_path.display());

    // TODO(marcua): Figure out how to pass path to program command
    cmd.args(["--bindmount_ro", "/home/marcua/ayb/src/hosted_db_runner/target/release/ayb-hosted-db-runner:/tmp/ayb-hosted-db-runner"]);
    cmd.args(["--bindmount", &db_file_mapping]);

    let mut child = cmd
        .arg("--")
        .arg("/tmp/ayb-hosted-db-runner")
        .arg(tmp_db_path)
        .arg(query)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    let stdout_reader = BufReader::new(child.stdout.take().unwrap());
    let stderr_reader = BufReader::new(child.stderr.take().unwrap());

    let output = child.wait_with_output().await?;

    // read stdout and stderr from process
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    stdout_reader
        .take(1024 * 1024 * 1024)
        .read_to_end(&mut stdout)
        .await?;
    stderr_reader
        .take(1024 * 1024 * 1024)
        .read_to_end(&mut stderr)
        .await?;
    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    let stderr = String::from_utf8_lossy(&stderr).into_owned();

    Ok(RunResult {
        status: output.status.code().ok_or(AybError {
            message: "Process exited with signal".to_string(),
        })?,
        stdout,
        stderr,
    })
}
