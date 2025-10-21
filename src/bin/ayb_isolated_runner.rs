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

/// This binary runs queries against a database and returns the
/// result in QueryResults format.
///
/// One-shot mode:
/// $ ayb_isolated_runner database.sqlite [0=read-only|1=read-write] SELECT xyz FROM ...
///
/// Daemon mode:
/// $ ayb_isolated_runner --daemon database.sqlite
/// Then send line-delimited JSON requests via stdin:
/// {"query":"SELECT * FROM x","query_mode":0}
///
/// This command is meant to be run inside a sandbox that isolates
/// parallel invocations of the command from accessing each
/// others' data, memory, and resources. That sandbox can be found
/// in src/hosted_db/sandbox.rs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--daemon" {
        // Daemon mode: read queries from stdin
        let db_file = PathBuf::from(&args[2]);
        run_daemon_mode(db_file)?;
    } else {
        // One-shot mode: execute single query from args
        let db_file = &args[1];
        let query_mode = QueryMode::try_from(
            args[2]
                .parse::<i16>()
                .expect("query mode should be an integer"),
        )
        .expect("query mode should be 0 or 1");
        let query = (args[3..]).to_vec();
        let result = query_sqlite(&PathBuf::from(db_file), &query.join(" "), false, query_mode);
        match result {
            Ok(result) => println!("{}", serde_json::to_string(&result)?),
            Err(error) => eprintln!("{}", serde_json::to_string(&error)?),
        }
    }
    Ok(())
}

fn run_daemon_mode(db_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
