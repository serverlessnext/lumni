mod about;
mod fallback;
mod home;
mod login;
mod logout;
mod object_stores;
mod users;

pub use about::About;
pub use fallback::Redirect;
pub use home::Home;
pub use login::Login;
pub use logout::Logout;
pub use object_stores::config::ObjectStoresId;
pub use object_stores::page::ObjectStores;
pub use users::config::UserId;
