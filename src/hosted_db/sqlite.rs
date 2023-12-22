use crate::error::AybError;
use crate::hosted_db::{sandbox::run_in_sandbox, QueryResult};
use crate::http::structs::AybConfigIsolation;
use ayb_hosted_db_runner::query_sqlite;
use serde_json;
use std::path::{Path, PathBuf};

pub async fn run_sqlite_query(
    path: &PathBuf,
    query: &str,
    isolation: &Option<AybConfigIsolation>,
) -> Result<QueryResult, AybError> {
    match isolation {
        Some(isolation) => {
            let result = run_in_sandbox(Path::new(&isolation.nsjail_path), path, query).await?;

            if result.stderr.len() > 0 {
                let error: AybError = serde_json::from_str(&result.stderr)?;
                Err(error)
            } else if result.status != 0 {
                Err(AybError {
                    message: format!(
                        "Error status from sandboxed query runner: {}",
                        result.status.to_string()
                    ),
                })
            } else if result.stdout.len() > 0 {
                let query_result: QueryResult = serde_json::from_str(&result.stdout)?;
                Ok(query_result)
            } else {
                Err(AybError {
                    message: "No results from sandboxed query runner".to_string(),
                })
            }
        }
        None => Ok(query_sqlite(path, query)?.into()),
    }
}
