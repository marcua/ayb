use ayb::ayb_db::models::DBType;
use ayb::hosted_db::engine::DbEngine;
use ayb::hosted_db::sandbox::apply_sandbox;
use ayb::hosted_db::{engine_for, QueryMode};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
struct QueryRequest {
    query: String,
    query_mode: i16,
}

/// This binary runs as a persistent daemon that executes queries
/// against a database and returns results in QueryResult format.
///
/// Usage:
/// $ ayb_query_daemon <database_file> <db_type>
///
/// The daemon reads line-delimited JSON requests from stdin:
/// {"query":"SELECT * FROM x","query_mode":[0=read-only|1=read-write]}
///
/// And writes line-delimited JSON responses to stdout.
///
/// At startup the daemon applies as much sandboxing as the host
/// supports (Landlock filesystem/network restrictions, setrlimit
/// resource limits) before processing any queries. The ayb server
/// detects the host's isolation capabilities at startup and prints
/// a prominent warning about any elements it cannot enforce.
/// See src/hosted_db/sandbox.rs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (db_file, db_type) = parse_args(&args)?;

    let engine = engine_for(&db_type);
    apply_sandbox(&db_file)?;

    run(db_file, engine)
}

fn parse_args(args: &[String]) -> Result<(PathBuf, DBType), Box<dyn std::error::Error>> {
    match args.len() {
        3 => {
            let db_type = DBType::from_str(&args[2])?;
            Ok((PathBuf::from(&args[1]), db_type))
        }
        _ => {
            eprintln!("Usage: ayb_query_daemon <database_file> <db_type>");
            std::process::exit(1);
        }
    }
}

fn run(db_file: PathBuf, engine: Box<dyn DbEngine>) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        let request: QueryRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let error_response = serde_json::json!({
                    "error": format!("Failed to parse request: {}", e)
                });
                writeln!(stdout, "{error_response}")?;
                stdout.flush()?;
                continue;
            }
        };

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

        let result = engine.query(&db_file, &request.query, false, query_mode);

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
