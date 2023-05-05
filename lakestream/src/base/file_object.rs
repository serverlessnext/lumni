use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

use futures::Future;
use serde::Deserialize;

use crate::base::callback_wrapper::CallbackItem;
use crate::utils::formatters::{bytes_human_readable, time_human_readable};

type BoxedAsyncCallback = Box<
    dyn Fn(&[FileObject]) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub struct FileObjectVec {
    file_objects: Vec<FileObject>,
    callback: Option<BoxedAsyncCallback>,
}

impl FileObjectVec {
    pub fn new(callback: Option<BoxedAsyncCallback>) -> Self {
        Self {
            file_objects: Vec::new(),
            callback,
        }
    }
    pub fn into_inner(self) -> Vec<FileObject> {
        self.file_objects
    }

    pub async fn extend_async<T: IntoIterator<Item = FileObject>>(
        &mut self,
        iter: T,
    ) {
        let new_file_objects: Vec<FileObject> = iter.into_iter().collect();
        if let Some(callback) = &self.callback {
            let fut = (callback)(&new_file_objects);
            fut.await;
        }

        self.file_objects.extend(new_file_objects);
    }
}

impl Deref for FileObjectVec {
    type Target = Vec<FileObject>;

    fn deref(&self) -> &Self::Target {
        &self.file_objects
    }
}

impl DerefMut for FileObjectVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file_objects
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileObject {
    name: String,
    size: u64,
    modified: Option<u64>,
    tags: Option<HashMap<String, String>>,
}

impl FileObject {
    pub fn new(
        name: String,
        size: u64,
        modified: Option<u64>,
        tags: Option<HashMap<String, String>>,
    ) -> Self {
        FileObject {
            name,
            size,
            modified,
            tags,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn modified(&self) -> Option<u64> {
        self.modified
    }

    pub fn tags(&self) -> &Option<HashMap<String, String>> {
        &self.tags
    }

    pub fn println_path(&self) -> String {
        self.printable(true)
    }

    fn printable(&self, full_path: bool) -> String {
        let name_without_trailing_slash = self.name.trim_end_matches('/');

        let mut name_to_print = if full_path {
            name_without_trailing_slash.to_string()
        } else {
            name_without_trailing_slash
                .split('/')
                .last()
                .unwrap_or(name_without_trailing_slash)
                .to_string()
        };

        if self.name.ends_with('/') {
            name_to_print.push('/');
        }

        format!(
            "{:8} {} {}",
            bytes_human_readable(self.size()),
            if let Some(modified) = self.modified() {
                time_human_readable(modified)
            } else {
                "PRE".to_string()
            },
            name_to_print
        )
    }
}

impl CallbackItem for FileObject {
    fn println_path(&self) -> String {
        self.println_path()
    }
}
