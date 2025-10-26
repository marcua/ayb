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
    allow_unsafe: bool,
}

/// This binary runs as a persistent daemon that executes queries
/// against a database and returns results in QueryResult format.
///
/// Usage:
/// $ ayb_isolated_runner database.sqlite
///
/// The daemon reads line-delimited JSON requests from stdin:
/// {"query":"SELECT * FROM x","query_mode":[0=read-only|1=read-write],"allow_unsafe":false}
///
/// And writes line-delimited JSON responses to stdout.
///
/// This command is meant to be run inside a sandbox that isolates
/// parallel invocations from accessing each other's data, memory,
/// and resources. That sandbox can be found in src/hosted_db/sandbox.rs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ayb_isolated_runner <database.sqlite>");
        std::process::exit(1);
    }

    let db_file = PathBuf::from(&args[1]);
    run(db_file)
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
                let error_msg = format!("{{\"error\":\"Failed to parse request: {}\"}}", e);
                writeln!(stdout, "{}", error_msg)?;
                stdout.flush()?;
                continue;
            }
        };

        // Convert query_mode to enum
        let query_mode = match QueryMode::try_from(request.query_mode) {
            Ok(mode) => mode,
            Err(_) => {
                let error_msg = "{\"error\":\"Invalid query_mode, must be 0 or 1\"}";
                writeln!(stdout, "{}", error_msg)?;
                stdout.flush()?;
                continue;
            }
        };

        // Execute the query
        let result = query_sqlite(&db_file, &request.query, request.allow_unsafe, query_mode);

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
