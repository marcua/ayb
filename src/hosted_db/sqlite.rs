use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::{QueryMode, QueryResult};
use rusqlite;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use std::path::PathBuf;

/// `allow_unsafe` disables features that prevent abuse but also
/// prevent backups/snapshots. The only known use case in the codebase
/// is for snapshots.
pub fn query_sqlite(
    path: &PathBuf,
    query: &str,
    allow_unsafe: bool,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    // The flags below are the default `open` flags in `rusqlite`
    // except for `..READ_ONLY` and `..READ_WRITE`.
    let mut open_flags =
        rusqlite::OpenFlags::SQLITE_OPEN_URI | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    open_flags |= match query_mode {
        QueryMode::ReadOnly => rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        QueryMode::ReadWrite => {
            rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        }
    };
    let conn = rusqlite::Connection::open_with_flags(path, open_flags)?;

    // Set busy timeout to 5 seconds to handle concurrent access
    conn.pragma_update(None, "busy_timeout", 5000)?;

    // Configure SQLite for optimal ayb usage
    if matches!(query_mode, QueryMode::ReadWrite) {
        // Enable WAL (Write-Ahead Logging) mode for better concurrency and performance.
        // This operation is idempotent and will convert non-WAL DBs to WAL ones.
        let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;

        // Set synchronous mode to FULL for maximum durability
        conn.pragma_update(None, "synchronous", "FULL")?;

        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", true)?;
    }

    if !allow_unsafe {
        // Disable the usage of ATTACH
        // https://www.sqlite.org/lang_attach.html
        conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
        // Prevent queries from deliberately corrupting the database
        // https://www.sqlite.org/c3ref/c_dbconfig_defensive.html
        conn.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)?;

        // Apply SQLite authorizer as defense-in-depth
        // This blocks dangerous operations even if other protections fail
        // Note: rusqlite 0.27 uses AuthContext with action() method
        use rusqlite::hooks::{AuthAction, Authorization};
        conn.authorizer(Some(|ctx: rusqlite::hooks::AuthContext| -> Authorization {
            match ctx.action {
                // Block ATTACH DATABASE - critical for multi-tenant isolation
                // This is redundant with SQLITE_LIMIT_ATTACHED=0 but provides defense-in-depth
                AuthAction::Attach { .. } => Authorization::Deny,

                // Block DETACH DATABASE
                AuthAction::Detach { .. } => Authorization::Deny,

                // Block function calls that could be dangerous
                AuthAction::Function { function_name } => {
                    // Block load_extension
                    if function_name.eq_ignore_ascii_case("load_extension") {
                        Authorization::Deny
                    } else {
                        // Allow all other functions
                        Authorization::Allow
                    }
                }

                // Block PRAGMA commands except safe ones
                AuthAction::Pragma {
                    pragma_name,
                    pragma_value: _,
                } => {
                    match pragma_name.to_lowercase().as_str() {
                        // Safe read-only PRAGMAs
                        "table_info" | "table_xinfo" | "table_list" | "index_info"
                        | "index_list" | "index_xinfo" | "database_list" | "foreign_key_list"
                        | "foreign_key_check" | "quick_check" | "integrity_check" | "encoding"
                        | "page_count" | "page_size" | "max_page_count" | "freelist_count"
                        | "schema_version" | "user_version" | "application_id" | "data_version"
                        | "compile_options" | "collation_list" | "module_list"
                        | "function_list" => Authorization::Allow,
                        // Safe runtime PRAGMAs
                        "busy_timeout"
                        | "cache_size"
                        | "case_sensitive_like"
                        | "count_changes"
                        | "foreign_keys"
                        | "ignore_check_constraints"
                        | "recursive_triggers"
                        | "reverse_unordered_selects"
                        | "query_only"
                        | "read_uncommitted"
                        | "synchronous"
                        | "temp_store" => Authorization::Allow,
                        // Journal mode is needed for WAL
                        "journal_mode" | "wal_checkpoint" | "wal_autocheckpoint" => {
                            Authorization::Allow
                        }
                        // Block dangerous PRAGMAs
                        _ => Authorization::Deny,
                    }
                }

                // Allow all other actions (SELECT, INSERT, UPDATE, DELETE, etc.)
                _ => Authorization::Allow,
            }
        }));
    }

    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut fields: Vec<String> = Vec::new();
    for column_index in 0..num_columns {
        fields.push(String::from(prepared.column_name(column_index)?))
    }

    let mut rows = prepared.query([])?;
    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next().map_err(|err| match err {
        rusqlite::Error::SqliteFailure(ref code, _)
            if code.code == rusqlite::ErrorCode::ReadOnly && code.extended_code == 8 =>
        {
            AybError::NoWriteAccessError {
                message: "Attempted to write to database while in read-only mode".to_string(),
            }
        }
        _ => AybError::from(err),
    })? {
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

/// Run `query` against the database at `path` using an isolated daemon process.
/// The daemon automatically applies Landlock, rlimits, and cgroups for isolation.
pub async fn potentially_isolated_sqlite_query(
    daemon_registry: &DaemonRegistry,
    path: &PathBuf,
    query: &str,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    daemon_registry.execute_query(path, query, query_mode).await
}
