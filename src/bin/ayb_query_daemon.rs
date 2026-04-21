use ayb::hosted_db::sandbox::apply_sandbox;
use ayb::hosted_db::sqlite::query_sqlite;
use ayb::hosted_db::QueryMode;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
struct QueryRequest {
    query: String,
    query_mode: i16,
}

/// This binary runs as a persistent daemon that executes queries
/// against a database and returns results in QueryResult format.
///
/// Usage:
/// $ ayb_query_daemon <database.sqlite>
///
/// The daemon reads line-delimited JSON requests from stdin:
/// {"query":"SELECT * FROM x","query_mode":[0=read-only|1=read-write]}
///
/// And writes line-delimited JSON responses to stdout.
///
/// The daemon always applies Landlock filesystem and network restrictions
/// plus resource limits (setrlimit) to itself before processing any
/// queries. On platforms or kernels where Landlock is unavailable, a
/// loud warning is printed to stderr and the daemon runs without
/// isolation. See src/hosted_db/sandbox.rs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let db_file = parse_args(&args)?;

    apply_sandbox(&db_file)?;

    // Temporary: verify Landlock blocks access outside the sandbox.
    // On Linux with Landlock enforced, this read returns
    // PermissionDenied and we panic — proving the sandbox is active.
    // On macOS (no Landlock), the read succeeds and the daemon
    // continues normally. Revert after confirming.
    match std::fs::read_to_string("/etc/hostname") {
        Ok(_) => eprintln!("SANDBOX TEST: /etc/hostname was readable (no Landlock on this host)"),
        Err(e) => panic!("SANDBOX TEST: Landlock blocked /etc/hostname read as expected: {e}"),
    }

    run(db_file)
}

fn parse_args(args: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match args.len() {
        2 => Ok(PathBuf::from(&args[1])),
        _ => {
            eprintln!("Usage: ayb_query_daemon <database.sqlite>");
            std::process::exit(1);
        }
    }
}

fn run(db_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        // Parse the query request
        let request: QueryRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                // Send error response and continue
                let error_response = serde_json::json!({
                    "error": format!("Failed to parse request: {}", e)
                });
                writeln!(stdout, "{error_response}")?;
                stdout.flush()?;
                continue;
            }
        };

        // Convert query_mode to enum
        let query_mode = match QueryMode::try_from(request.query_mode) {
            Ok(mode) => mode,
            Err(_) => {
                let error_response = serde_json::json!({
                    "error": "Invalid query_mode, must be 0 or 1"
                });
                writeln!(stdout, "{error_response}")?;
                stdout.flush()?;
                continue;
            }
        };

        // Execute the query
        let result = query_sqlite(&db_file, &request.query, false, query_mode);

        // Send response
        match result {
            Ok(result) => {
                writeln!(stdout, "{}", serde_json::to_string(&result)?)?;
            }
            Err(error) => {
                writeln!(stdout, "{}", serde_json::to_string(&error)?)?;
            }
        }
        stdout.flush()?;
    }

    Ok(())
}
