use ayb::hosted_db::sqlite::query_sqlite;
use std::env;
use std::path::PathBuf;

/// This binary runs a query against a database and returns the
/// result in QueryResults format. To run it, you would type:
/// $ ayb_isolated_runner database.sqlite SELECT xyz FROM ...
///
/// This command is meant to be run inside a sandbox that isolates
/// parallel invocations of the command from accessing each
/// others' data, memory, and resources. That sandbox can be found
/// in src/hosted_db/sandbox.rs.
fn main() -> Result<(), serde_json::Error> {
    let args: Vec<String> = env::args().collect();
    let db_file = &args[1];
    let query = (args[2..]).to_vec();
    let result = query_sqlite(&PathBuf::from(db_file), &query.join(" "), false);
    match result {
        Ok(result) => println!("{}", serde_json::to_string(&result)?),
        Err(error) => eprintln!("{}", serde_json::to_string(&error)?),
    }
    Ok(())
}
