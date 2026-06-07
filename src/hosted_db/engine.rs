use crate::error::AybError;
use crate::hosted_db::{QueryMode, QueryResult};
use crate::server::config::AybConfigSnapshots;
use std::path::Path;

pub trait DbEngine: Send + Sync {
    fn query(
        &self,
        path: &Path,
        query: &str,
        allow_unsafe: bool,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError>;

    fn create_snapshot(
        &self,
        config: &AybConfigSnapshots,
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<(), AybError>;

    fn db_type_str(&self) -> &'static str;
}
