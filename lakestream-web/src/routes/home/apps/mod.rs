mod list_view;
mod page;
mod app;
mod app_view;
mod templates;

pub use page::{Apps, AppConfiguration};
pub use app::AppId;
pub use app_view::AppView;
use templates::*;

use crate::builders::ElementBuilder;

pub trait ProfileList {
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<ElementBuilder>;
}

pub enum Profile {
    ObjectStoreS3(ObjectStoreS3),
    Environment(Environment),
}

impl ProfileList for Profile {
    fn name(&self) -> String {
        match self {
            Profile::ObjectStoreS3(c) => c.name(),
            Profile::Environment(c) => c.name(),
        }
    }

    fn id(&self) -> String {
        match self {
            Profile::ObjectStoreS3(c) => c.id(),
            Profile::Environment(c) => c.id(),
        }
    }

    fn form_elements<S: Into<String>>(&self, name: S) -> Vec<ElementBuilder> {
        match self {
            Profile::ObjectStoreS3(c) => c.form_elements(name),
            Profile::Environment(c) => c.form_elements(name),
        }
    }
}
