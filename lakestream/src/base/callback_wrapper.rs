use std::pin::Pin;

use futures::Future;

use crate::FileObject;

type BoxedCallback = Box<dyn Fn(&[FileObject]) + Send + Sync + 'static>;
type BoxedAsyncCallback = Box<
    dyn Fn(&[FileObject]) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub enum CallbackWrapper {
    Sync(BoxedCallback),
    Async(BoxedAsyncCallback),
}

impl CallbackWrapper {
    pub fn create_sync<F>(func: F) -> Self
    where
        F: Fn(&[FileObject]) + Send + Sync + 'static,
    {
        CallbackWrapper::Sync(Box::new(func))
    }

    pub fn create_async<F, Fut>(func: F) -> Self
    where
        F: Fn(Vec<FileObject>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let wrapped_func = Self::wrap_async_fn(func);
        CallbackWrapper::Async(Box::new(wrapped_func))
    }

    fn wrap_async_fn<F, Fut>(
        func: F,
    ) -> impl Fn(
        &[FileObject],
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
           + Send
           + Sync
           + 'static
    where
        F: Fn(Vec<FileObject>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        move |file_objects: &[FileObject]| {
            let file_objects_cloned = file_objects.to_owned();
            let future = func(file_objects_cloned);
            Box::pin(future)
                as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        }
    }
}
