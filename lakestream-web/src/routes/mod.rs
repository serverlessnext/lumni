mod about;
mod fallback;
mod home;
mod login;
mod logout;

pub use about::About;
pub use fallback::Redirect;
pub use home::Home;
pub use login::Login;
pub use logout::Logout;
pub mod object_stores;
pub mod users;
