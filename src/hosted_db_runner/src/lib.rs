use rusqlite;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::string;
use std::vec::Vec;

#[derive(Serialize, Debug, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AybError {
    pub message: String,
}

impl From<std::io::Error> for AybError {
    fn from(cause: std::io::Error) -> Self {
        AybError {
            message: format!("IO error: {:?}", cause),
        }
    }
}

impl From<rusqlite::Error> for AybError {
    fn from(cause: rusqlite::Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<rusqlite::types::FromSqlError> for AybError {
    fn from(cause: rusqlite::types::FromSqlError) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<std::str::Utf8Error> for AybError {
    fn from(cause: std::str::Utf8Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<string::FromUtf8Error> for AybError {
    fn from(cause: string::FromUtf8Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

pub fn query_sqlite(path: &PathBuf, query: &str) -> Result<QueryResult, AybError> {
    let conn = rusqlite::Connection::open(path)?;

    // Disable the usage of ATTACH
    // https://www.sqlite.org/lang_attach.html
    conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    // Prevent queries from deliberately corrupting the database
    // https://www.sqlite.org/c3ref/c_dbconfig_defensive.html
    conn.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)?;

    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut fields: Vec<String> = Vec::new();
    for column_index in 0..num_columns {
        fields.push(String::from(prepared.column_name(column_index)?))
    }

    let mut rows = prepared.query([])?;
    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next()? {
        let mut result: Vec<Option<String>> = Vec::new();
        for column_index in 0..num_columns {
            let column_value = row.get_ref(column_index)?;
            result.push(match column_value {
                ValueRef::Null => None,
                ValueRef::Integer(i) => Some(i.to_string()),
                ValueRef::Real(f) => Some(f.to_string()),
                ValueRef::Text(_t) => Some(column_value.as_str()?.to_string()),
                ValueRef::Blob(_b) => {
                    Some(std::str::from_utf8(column_value.as_bytes()?)?.to_string())
                }
            });
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}
