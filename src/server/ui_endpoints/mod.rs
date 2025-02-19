mod confirm;
mod entity_details;
mod log_in;
mod register;
mod templates;

# AI! Make each of the endpoints below be imported as endpoint_name_endpoint. For example pub use confirm::confirm as confirm_endpoint;
pub use confirm::confirm;
pub use entity_details::entity_details;
pub use log_in::{log_in, log_in_submit};
pub use register::{register, register_submit};
pub use templates::{base_auth, base_content};
