mod display_user;
mod login;
mod register;
mod templates;

pub use display_user::display_user as display_user_route;
pub use login::{login_page as login_page_route, login_submit as login_submit_route};
pub use register::{register_page as register_page_route, register_submit as register_submit_route};
pub use templates::base_template;
