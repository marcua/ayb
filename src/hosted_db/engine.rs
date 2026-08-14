use crate::error::AybError;
use crate::hosted_db::{QueryMode, QueryResult};
use std::path::Path;

/// A hosted database engine (SQLite or DuckDB).
///
/// Implementations are stateless; `engine_for` hands out a shared
/// instance of each. Every method runs the engine's safety perimeter
/// (no extension loading, no external file/network access, no ATTACH),
/// so callers cannot ask for an unrestricted connection. Engines relax
/// those restrictions internally where a specific operation requires it
/// (e.g., snapshots need ATTACH) but that stays an implementation detail
/// rather than a parameter callers can pass.
pub trait DbEngine: Send + Sync {
    fn query(
        &self,
        path: &Path,
        query: &str,
        params: &[serde_json::Value],
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError>;

    /// Write a consistent copy of the database at `db_path` to
    /// `snapshot_path`, then verify the copy is readable.
    fn create_snapshot(&self, db_path: &Path, snapshot_path: &Path) -> Result<(), AybError>;
}
