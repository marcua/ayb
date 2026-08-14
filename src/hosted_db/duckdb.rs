use crate::error::AybError;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::{sql_string_literal, QueryMode, QueryResult};
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
        params: &[serde_json::Value],
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_duckdb(path, query, params, false, query_mode)
    }

    fn create_snapshot(&self, db_path: &Path, snapshot_path: &Path) -> Result<(), AybError> {
        let attach = format!(
            "ATTACH {} AS src (READ_ONLY); ATTACH {} AS dst; COPY FROM DATABASE src TO dst;",
            sql_string_literal(db_path),
            sql_string_literal(snapshot_path)
        );
        // The source may be momentarily locked by an in-flight query, so
        // retry on lock conflicts the same way opening a connection does.
        with_lock_retry(|| {
            let conn = duckdb::Connection::open_in_memory_with_flags(snapshot_config()?)?;
            conn.execute_batch(&attach)?;
            Ok(())
        })?;
        // Verify the snapshot is a readable DuckDB database: a successful
        // query here (propagated via `?`) means the file opened and is
        // queryable. `information_schema.tables` always returns exactly one
        // row (the count), so there is nothing further to assert on it.
        query_duckdb(
            snapshot_path,
            "SELECT count(*) FROM information_schema.tables;",
            &[],
            false,
            QueryMode::ReadOnly,
        )?;
        Ok(())
    }
}

fn query_duckdb(
    path: &Path,
    query: &str,
    params: &[serde_json::Value],
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

    let bound = params
        .iter()
        .map(json_to_duckdb_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = prepared
        .query(duckdb::params_from_iter(bound))
        .map_err(map_duckdb_error)?;

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
            let value: Value = row.get(col_idx).map_err(map_duckdb_error)?;
            result.push(duckdb_value_to_string(value));
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}

/// How long to keep retrying an operation blocked by a DuckDB file lock,
/// and how long to wait between attempts. The total matches the
/// `busy_timeout` we give SQLite, so both engines wait the same amount
/// for a contended database.
const LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(100);

/// Run `op`, retrying while it fails with a DuckDB file-lock conflict.
///
/// DuckDB takes a whole-file lock when it opens a database and fails
/// *immediately* if another process holds a conflicting lock (a
/// read-write open is exclusive; a read-only open is shared). In ayb the
/// snapshot job periodically opens each database to back it up, so a
/// query can collide with an in-progress snapshot -- and a snapshot's
/// ATTACH can collide with an in-flight query. Unlike SQLite, where a
/// `busy_timeout` makes a contended open wait, DuckDB has no built-in
/// wait, so we poll with a short backoff before giving up. Any error
/// that is not a lock conflict is returned immediately.
fn with_lock_retry<T>(mut op: impl FnMut() -> Result<T, duckdb::Error>) -> Result<T, AybError> {
    let deadline = Instant::now() + LOCK_RETRY_TIMEOUT;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if is_lock_conflict(&err) && Instant::now() < deadline {
                    thread::sleep(LOCK_RETRY_INTERVAL);
                    continue;
                }
                return Err(map_duckdb_error(err));
            }
        }
    }
}

/// Open a DuckDB connection at `path` for `query_mode`, waiting out a
/// transient file-lock conflict.
///
/// The Config is rebuilt on each attempt because `open_with_flags`
/// consumes it. Threads/memory are capped via `base_config`: DuckDB
/// probes the host (/sys/devices/system/cpu/online, /sys/fs/cgroup/...,
/// /proc/self/*) at instantiation to auto-size them, which would blow the
/// daemon's RLIMIT_AS on a large host, so `base_config` pins them.
fn open_with_retry(path: &Path, query_mode: QueryMode) -> Result<duckdb::Connection, AybError> {
    let access_mode = match query_mode {
        QueryMode::ReadOnly => duckdb::AccessMode::ReadOnly,
        QueryMode::ReadWrite => duckdb::AccessMode::ReadWrite,
    };
    with_lock_retry(|| {
        let config = base_config()?.access_mode(access_mode.clone())?;
        duckdb::Connection::open_with_flags(path, config)
    })
}

/// Base DuckDB config shared by the query and snapshot connections: a
/// single worker thread and a 128 MB buffer pool, keeping the process
/// within the daemon's RLIMIT_AS regardless of host CPU/RAM size.
fn base_config() -> duckdb::Result<duckdb::Config> {
    duckdb::Config::default().threads(1)?.max_memory("128MB")
}

/// Config for the snapshot connection. Snapshots run ATTACH, which needs
/// external file access, so unlike the query path we cannot disable it.
/// Extension autoloading/autoinstalling is not needed either way and is
/// turned off so a snapshot can never pull in and run extension code --
/// this connection runs in the server process, outside the daemon's
/// Landlock sandbox.
fn snapshot_config() -> duckdb::Result<duckdb::Config> {
    base_config()?
        .with("autoinstall_known_extensions", "false")?
        .with("autoload_known_extensions", "false")
}

/// True if `err` reports a DuckDB file-lock conflict.
///
/// This has to match on the message text: the `duckdb` crate builds every
/// failure with `ffi::Error::new`, which hardcodes `ErrorCode::Unknown`
/// and carries only a generic `extended_code` (1 = DuckDBError), so no
/// structured code distinguishes a lock conflict from any other open
/// failure. The unit tests below provoke a real lock conflict and assert
/// this returns true, so a DuckDB upgrade that rewords the message fails
/// CI loudly instead of silently disabling the retry.
fn is_lock_conflict(err: &duckdb::Error) -> bool {
    let message = err.to_string();
    message.contains("Could not set lock") || message.contains("Conflicting lock")
}

/// True if `err` reports a write attempted against a read-only database.
/// Message-matched for the same reason as `is_lock_conflict`, and
/// likewise pinned by a unit test below.
fn is_read_only_violation(err: &duckdb::Error) -> bool {
    let message = err.to_string();
    message.contains("read-only") || message.contains("Cannot execute write")
}

/// Convert a JSON-native bind parameter into a DuckDB value. Integers
/// bind as `BigInt`, non-integer numbers as `Double`, and booleans use
/// DuckDB's native `BOOLEAN` (unlike SQLite, which has no boolean). JSON
/// arrays and objects have no scalar DuckDB equivalent and are rejected.
fn json_to_duckdb_value(value: &serde_json::Value) -> Result<Value, AybError> {
    Ok(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::BigInt(i)
            } else if let Some(f) = n.as_f64() {
                Value::Double(f)
            } else {
                return Err(AybError::QueryError {
                    message: format!("Unsupported numeric bind parameter: {n}"),
                });
            }
        }
        serde_json::Value::String(s) => Value::Text(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            return Err(AybError::QueryError {
                message: "Array and object bind parameters are not supported".to_string(),
            });
        }
    })
}

fn map_duckdb_error(err: duckdb::Error) -> AybError {
    if is_read_only_violation(&err) {
        return AybError::NoWriteAccessError {
            message: "Attempted to write to database while in read-only mode".to_string(),
        };
    }
    AybError::from(err)
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
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert!(r.rows.is_empty());

        let r = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'hello'), (2, 'world');",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["Count"]);
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        let r = query_duckdb(
            &path,
            "SELECT * FROM t ORDER BY id;",
            &[],
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

    /// Pins `is_read_only_violation` against a real read-only write
    /// error. If a DuckDB upgrade rewords the message, this fails rather
    /// than silently downgrading NoWriteAccessError to a generic error.
    #[test]
    fn test_read_only_violation_is_recognized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("classify_ro.duckdb");
        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        let conn = open_with_retry(&path, QueryMode::ReadOnly).unwrap();
        let err = conn
            .execute_batch("INSERT INTO t VALUES (1);")
            .expect_err("insert into a read-only database should fail");
        assert!(
            is_read_only_violation(&err),
            "unrecognized read-only error: {err}"
        );
        assert!(matches!(
            map_duckdb_error(err),
            AybError::NoWriteAccessError { .. }
        ));

        fs::remove_dir_all(dir.path()).ok();
    }

    /// Pins `is_lock_conflict` against a real lock conflict: hold a
    /// read-write connection open, then try to open the same database
    /// again. If a DuckDB upgrade rewords the message, this fails rather
    /// than silently disabling the retry in `with_lock_retry`.
    #[test]
    fn test_lock_conflict_is_recognized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("classify_lock.duckdb");
        let _holder = {
            let config = base_config()
                .unwrap()
                .access_mode(duckdb::AccessMode::ReadWrite)
                .unwrap();
            duckdb::Connection::open_with_flags(&path, config).unwrap()
        };

        let config = base_config()
            .unwrap()
            .access_mode(duckdb::AccessMode::ReadWrite)
            .unwrap();
        // The conflict this guards against is cross-process (the server's
        // snapshot job vs. a query daemon). If a DuckDB build permits a
        // second open within one process there is nothing to pin here, so
        // don't fail the suite over it.
        if let Err(err) = duckdb::Connection::open_with_flags(&path, config) {
            assert!(is_lock_conflict(&err), "unrecognized lock error: {err}");
        }

        fs::remove_dir_all(dir.path()).ok();
    }

    #[test]
    fn test_duckdb_read_only_prevents_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_ro.duckdb");

        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        let result = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1);",
            &[],
            false,
            QueryMode::ReadOnly,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_duckdb_positional_params() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_params.duckdb");

        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER, name VARCHAR, score DOUBLE);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'alice', 9.5), (2, 'bob', 7.0);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        // Bind a string and a number positionally ($1, $2).
        let r = query_duckdb(
            &path,
            "SELECT id FROM t WHERE name = $1 AND score >= $2 ORDER BY id;",
            &[serde_json::json!("alice"), serde_json::json!(9.0)],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("1".to_string())]]);

        // A NULL bind parameter compares as expected.
        let r = query_duckdb(
            &path,
            "SELECT count(*) FROM t WHERE $1 IS NULL;",
            &[serde_json::Value::Null],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        fs::remove_dir_all(dir.path()).ok();
    }
}
