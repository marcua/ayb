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
use serde::{Deserialize, Serialize};
use std::env::current_exe;
use std::fs::canonicalize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
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
        .args(["--bindmount_ro", "/usr:/usr"]);

    // Set resource limits for the process. In the future, we will
    // allow entities to control the resources they dedicate to
    // different databases/queries.
    cmd.args(["--mount", "none:/tmp:tmpfs:size=100000000"]) // ~95 MB tmpfs
        .args(["--max_cpus", "1"]) // One CPU
        .args(["--rlimit_as", "64"]) // 64 MB memory limit
        .args(["--time_limit", "10"]) // 10 second maximum run
        .args(["--rlimit_fsize", "75"]) // 75 MB file size limit
        .args(["--rlimit_nofile", "10"]) // 10 files maximum
        .args(["--rlimit_nproc", "2"]); // 2 processes maximum

    // Generate a /local/path/to/file:/tmp/file mapping.
    let absolute_db_path = canonicalize(db_path)?;
    let db_file_name = absolute_db_path
        .file_name()
        .ok_or(AybError::Other {
            message: format!("Could not parse file name from path: {}", absolute_db_path.display()),
        })?
        .to_str()
        .ok_or(AybError::Other {
            message: format!("Could not convert path to string: {}", absolute_db_path.display()),
        })?;
    let tmp_db_path = Path::new("/tmp").join(db_file_name);
    let db_file_mapping = format!("{}:{}", absolute_db_path.display(), tmp_db_path.display());
    cmd.args(["--bindmount", &db_file_mapping]);

    // Generate a /local/path/to/ayb_isolated_runner:/tmp/ayb_isolated_runner mapping.
    // We assume `ayb` and `ayb_isolated_runner` will always be in the same directory,
    // so we see what the path to the current `ayb` executable is to build the path.
    let ayb_path = current_exe()?;
    let isolated_runner_path = ayb_path
        .parent()
        .ok_or(AybError::Other {
            message: format!(
                "Unable to find parent directory of ayb from {}",
                ayb_path.display()
            ),
        })?
        .join("ayb_isolated_runner");
    cmd.args([
        "--bindmount_ro",
        &format!(
            "{}:/tmp/ayb_isolated_runner",
            isolated_runner_path.display()
        ),
    ]);

    let mut child = cmd
        .arg("--")
        .arg("/tmp/ayb_isolated_runner")
        .arg(tmp_db_path)
        .arg(query)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdout_reader = BufReader::new(child.stdout.take().unwrap());
    let mut stderr_reader = BufReader::new(child.stderr.take().unwrap());

    let output = child.wait_with_output().await?;

    // read stdout and stderr from process
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    stdout_reader
        .read_to_end(&mut stdout)
        .await?;
    stderr_reader
        .read_to_end(&mut stderr)
        .await?;
    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    let stderr = String::from_utf8_lossy(&stderr).into_owned();

    Ok(RunResult {
        status: output.status.code().ok_or(AybError::Other {
            message: "Process exited with signal".to_string(),
        })?,
        stdout,
        stderr,
    })
}
