mod confirm;
mod create_database;
mod entity_details;
mod list_snapshots;
mod log_in;
mod query;
mod register;
mod restore_snapshot;
mod update_profile;

pub use confirm::confirm as confirm_endpoint;
pub use create_database::create_database as create_db_endpoint;
pub use entity_details::entity_details as entity_details_endpoint;
pub use list_snapshots::list_snapshots as list_snapshots_endpoint;
pub use log_in::log_in as log_in_endpoint;
pub use query::query as query_endpoint;
pub use register::register as register_endpoint;
pub use restore_snapshot::restore_snapshot as restore_snapshot_endpoint;
pub use update_profile::update_profile as update_profile_endpoint;
