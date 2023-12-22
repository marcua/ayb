use ayb_hosted_db_runner::{query_sqlite};
use serde_json;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), serde_json::Error> {
    let args: Vec<String> = env::args().collect();
    let db_file = &args[1];
    let query = (&args[2..]).to_vec();
    let result = query_sqlite(&PathBuf::from(db_file), &query.join(" "));
    match result {
        Ok(result) => println!("{}", serde_json::to_string(&result)?),
        Err(error) => eprintln!("{}", serde_json::to_string(&error)?),
    }
    Ok(())
}
