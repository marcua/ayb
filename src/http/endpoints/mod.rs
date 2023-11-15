mod confirm;
mod create_database;
mod log_in;
mod register;
mod query;

pub use confirm::confirm as confirm_endpoint;
pub use create_database::create_database as create_db_endpoint;
pub use log_in::log_in as log_in_endpoint;
pub use register::register as register_endpoint;
pub use query::query as query_endpoint;
