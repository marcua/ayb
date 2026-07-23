use crate::error::AybError;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::{QueryMode, QueryResult};
use crate::server::config::AybConfigSnapshots;
use duckdb::types::{TimeUnit, Value};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub struct DuckdbEngine;

impl DbEngine for DuckdbEngine {
    fn query(
        &self,
        path: &Path,
        query: &str,
        allow_unsafe: bool,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_duckdb(path, query, allow_unsafe, query_mode)
    }

    fn create_snapshot(
        &self,
        _config: &AybConfigSnapshots,
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<(), AybError> {
        // Copy the database into a fresh snapshot file via an in-memory
        // connection that attaches both sides with explicit aliases.
        // Opening the source directly and running "COPY FROM DATABASE main"
        // fails with `Catalog "main" does not exist` because the source
        // catalog is named after the file, not "main". Opening the source
        // read-only would also make the whole instance (including the
        // attached destination) read-only, breaking the COPY. With an
        // in-memory read-write main, the source is attached READ_ONLY (so
        // it is never modified) and the destination read-write.
        let conn = duckdb::Connection::open_in_memory_with_flags(base_config()?)?;
        conn.execute_batch(&format!(
            "ATTACH '{}' AS src (READ_ONLY); ATTACH '{}' AS dst; \
             COPY FROM DATABASE src TO dst;",
            db_path.display(),
            snapshot_path.display()
        ))
        .map_err(map_duckdb_error)?;
        drop(conn);
        // Verify the snapshot is a readable DuckDB database: a successful
        // query here (propagated via `?`) means the file opened and is
        // queryable. `information_schema.tables` always returns exactly one
        // row (the count), so there is nothing further to assert on it.
        query_duckdb(
            snapshot_path,
            "SELECT count(*) FROM information_schema.tables;",
            false,
            QueryMode::ReadOnly,
        )?;
        Ok(())
    }

    fn db_type_str(&self) -> &'static str {
        "duckdb"
    }
}

fn query_duckdb(
    path: &Path,
    query: &str,
    allow_unsafe: bool,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    let conn = open_with_retry(path, query_mode)?;

    if !allow_unsafe {
        // Disable extension install/load and external (file/network)
        // access, then lock the configuration so a query can't re-enable
        // them. This is the same safety perimeter as SQLite's ATTACH/
        // defensive settings.
        conn.execute_batch(
            "SET autoinstall_known_extensions=false;
             SET autoload_known_extensions=false;
             SET enable_external_access=false;
             SET lock_configuration=true;",
        )?;
    }

    let mut prepared = conn.prepare(query).map_err(map_duckdb_error)?;

    let mut rows = prepared.query([]).map_err(map_duckdb_error)?;

    // Read column metadata inside a scoped borrow so the mutable
    // rows.next() below is free to borrow `rows` again. Return an error
    // rather than unwrap()ing if there is no result statement.
    let (num_columns, fields) = {
        let statement = rows.as_ref().ok_or_else(|| AybError::Other {
            message: "DuckDB query produced no result statement".to_string(),
        })?;
        let num_columns = statement.column_count();
        let mut fields: Vec<String> = Vec::with_capacity(num_columns);
        for i in 0..num_columns {
            fields.push(statement.column_name(i)?.to_string());
        }
        (num_columns, fields)
    };

    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next().map_err(map_duckdb_error)? {
        let mut result: Vec<Option<String>> = Vec::new();
        for col_idx in 0..num_columns {
            let value: duckdb::Result<Value> = row.get(col_idx);
            match value {
                Ok(val) => result.push(duckdb_value_to_string(val)),
                Err(_) => result.push(None),
            }
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}

/// Open a DuckDB connection at `path`, retrying briefly on a file-lock
/// conflict.
///
/// DuckDB takes a whole-file lock when it opens a database and fails
/// *immediately* if another process holds a conflicting lock (a
/// read-write open is exclusive; a read-only open is shared). In ayb the
/// snapshot daemon periodically opens each database to back it up, so a
/// query can collide with an in-progress snapshot (or another query) and
/// get an `IO Error: Could not set lock on file ...` failure. Unlike
/// SQLite -- where we set a `busy_timeout` so a contended open waits --
/// DuckDB has no built-in wait, so we poll with a short backoff for a few
/// seconds before giving up. This keeps transient contention from
/// surfacing to callers as a query error.
///
/// The Config is rebuilt each attempt because `open_with_flags` consumes
/// it. Threads/memory are capped via `base_config`: DuckDB probes the
/// host (/sys/devices/system/cpu/online, /sys/fs/cgroup/..., /proc/self/*)
/// at instantiation to auto-size them, which would blow the daemon's
/// RLIMIT_AS on a large host, so `base_config` pins them to fixed values.
fn open_with_retry(path: &Path, query_mode: QueryMode) -> Result<duckdb::Connection, AybError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let config = base_config()?
            .access_mode(match query_mode {
                QueryMode::ReadOnly => duckdb::AccessMode::ReadOnly,
                QueryMode::ReadWrite => duckdb::AccessMode::ReadWrite,
            })
            .map_err(config_err)?;

        match duckdb::Connection::open_with_flags(path, config) {
            Ok(conn) => return Ok(conn),
            Err(err) => {
                let message = err.to_string();
                let lock_conflict =
                    message.contains("Could not set lock") || message.contains("Conflicting lock");
                if lock_conflict && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                return Err(AybError::from(err));
            }
        }
    }
}

/// Base DuckDB config shared by the query and snapshot connections: a
/// single worker thread and a 128 MB buffer pool, keeping the process
/// within the daemon's RLIMIT_AS regardless of host CPU/RAM size.
fn base_config() -> Result<duckdb::Config, AybError> {
    duckdb::Config::default()
        .threads(1)
        .map_err(config_err)?
        .max_memory("128MB")
        .map_err(config_err)
}

fn config_err(e: duckdb::Error) -> AybError {
    AybError::Other {
        message: format!("DuckDB config error: {e}"),
    }
}

fn map_duckdb_error(err: duckdb::Error) -> AybError {
    match &err {
        duckdb::Error::DuckDBFailure(_, Some(msg))
            if msg.contains("read-only") || msg.contains("Cannot execute write") =>
        {
            AybError::NoWriteAccessError {
                message: "Attempted to write to database while in read-only mode".to_string(),
            }
        }
        _ => AybError::from(err),
    }
}

/// Convert a DuckDB time-unit-tagged integer into microseconds.
fn duckdb_micros(unit: TimeUnit, value: i64) -> i64 {
    match unit {
        TimeUnit::Second => value.saturating_mul(1_000_000),
        TimeUnit::Millisecond => value.saturating_mul(1_000),
        TimeUnit::Microsecond => value,
        TimeUnit::Nanosecond => value / 1_000,
    }
}

fn duckdb_value_to_string(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Boolean(b) => Some(b.to_string()),
        Value::TinyInt(i) => Some(i.to_string()),
        Value::SmallInt(i) => Some(i.to_string()),
        Value::Int(i) => Some(i.to_string()),
        Value::BigInt(i) => Some(i.to_string()),
        Value::HugeInt(i) => Some(i.to_string()),
        Value::UTinyInt(i) => Some(i.to_string()),
        Value::USmallInt(i) => Some(i.to_string()),
        Value::UInt(i) => Some(i.to_string()),
        Value::UBigInt(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Double(f) => Some(f.to_string()),
        Value::Text(s) => Some(s),
        Value::Blob(b) => Some(String::from_utf8_lossy(&b).to_string()),
        // Temporal types are tagged integers (days/microseconds since the
        // Unix epoch or midnight). Render them as readable date/time strings
        // rather than the raw integers or Debug form. Fall back to the raw
        // value only if the timestamp is out of chrono's representable range.
        Value::Date32(days) => chrono::DateTime::from_timestamp((days as i64) * 86_400, 0)
            .map(|dt| dt.date_naive().to_string())
            .or_else(|| Some(days.to_string())),
        Value::Timestamp(unit, v) => {
            let micros = duckdb_micros(unit, v);
            chrono::DateTime::from_timestamp(
                micros.div_euclid(1_000_000),
                (micros.rem_euclid(1_000_000) as u32) * 1_000,
            )
            .map(|dt| dt.naive_utc().to_string())
            .or_else(|| Some(v.to_string()))
        }
        Value::Time64(unit, v) => {
            let micros = duckdb_micros(unit, v);
            chrono::NaiveTime::from_num_seconds_from_midnight_opt(
                micros.div_euclid(1_000_000) as u32,
                (micros.rem_euclid(1_000_000) as u32) * 1_000,
            )
            .map(|t| t.to_string())
            .or_else(|| Some(v.to_string()))
        }
        // Remaining types (lists, structs, maps, decimals, intervals, ...)
        // have no lossless scalar form; stringify with their Debug rendering.
        _ => Some(format!("{value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_duckdb_create_insert_select() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.duckdb");

        let r = query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER, name VARCHAR);",
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert!(r.rows.is_empty());

        let r = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'hello'), (2, 'world');",
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["Count"]);
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        let r = query_duckdb(
            &path,
            "SELECT * FROM t ORDER BY id;",
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["id", "name"]);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(
            r.rows[0],
            vec![Some("1".to_string()), Some("hello".to_string())]
        );
        assert_eq!(
            r.rows[1],
            vec![Some("2".to_string()), Some("world".to_string())]
        );

        fs::remove_dir_all(dir.path()).ok();
    }

    #[test]
    fn test_duckdb_read_only_prevents_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_ro.duckdb");

        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER);",
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        let result = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1);",
            false,
            QueryMode::ReadOnly,
        );
        assert!(result.is_err());
    }
}
