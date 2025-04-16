mod client;
mod confirm;
mod create_database;
mod database;
mod entity_details;
mod log_in;
mod query;
mod register;
mod templates;

pub use confirm::confirm as confirm_endpoint;
pub use create_database::create_database as create_database_endpoint;
pub use database::database as database_endpoint;
pub use entity_details::entity_details as entity_details_endpoint;
pub use log_in::{log_in as log_in_endpoint, log_in_submit as log_in_submit_endpoint};
pub use query::query as query_endpoint;
pub use register::{register as register_endpoint, register_submit as register_submit_endpoint};
pub use templates::{base_auth, base_content};
