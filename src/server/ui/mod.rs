mod confirm;
mod display_user;
mod login;
mod register;
mod templates;
mod web_details;

pub use confirm::confirm_page as confirm_page_route;
pub use display_user::display_user as display_user_route;
pub use login::{login_page as login_page_route, login_submit as login_submit_route};
pub use register::{
    register_page as register_page_route, register_submit as register_submit_route,
};
pub use templates::base_template;
pub use web_details::web_details_page as web_details_page_route;
