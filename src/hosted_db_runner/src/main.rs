use hosted_db_runner::{run_sqlite_query, AybError, QueryResult};
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    let db_file = &args[1];
    let query = (&args[2..]).to_vec();
    match run_sqlite_query(&PathBuf::from(db_file), &query.join(" ")) {
        Ok(result) => println!("{:?}", result),
        Err(error) => eprintln!("{:?}", error),
    }
}
