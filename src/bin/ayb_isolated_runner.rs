use ayb::hosted_db::sqlite::query_sqlite;
use ayb::hosted_db::QueryMode;
use std::env;
use std::path::PathBuf;

/// This binary runs a query against a database and returns the
/// result in QueryResults format. To run it, you would type:
/// $ ayb_isolated_runner database.sqlite [0=read-only|1=read-write] SELECT xyz FROM ...
///
/// This command is meant to be run inside a sandbox that isolates
/// parallel invocations of the command from accessing each
/// others' data, memory, and resources. That sandbox can be found
/// in src/hosted_db/sandbox.rs.
fn main() -> Result<(), serde_json::Error> {
    let args: Vec<String> = env::args().collect();
    let db_file = &args[1];
    let query_mode = QueryMode::try_from(
        args[2]
            .parse::<i16>()
            .expect("query mode should be an integer"),
    )
    .expect("query mode should be 0 or 1");
    let query = (args[3..]).to_vec();
    let result = query_sqlite(&PathBuf::from(db_file), &query.join(" "), false, query_mode);
    let query_mode2 = QueryMode::try_from(
        args[2]
            .parse::<i16>()
            .expect("query mode should be an integer"),
    )
    .expect("query mode should be 0 or 1");
    match result {
        Ok(result) => println!("{}", serde_json::to_string(&result)?),
        Err(error) => eprintln!(
            "{}---{}---{:?}---{:?}",
            serde_json::to_string(&error)?,
            db_file,
            query_mode2,
            query
        ),
    }
    Ok(())
}
