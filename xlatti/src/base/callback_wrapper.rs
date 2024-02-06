use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type SyncCallback<T> = Arc<dyn Fn(&[T]) + Send + Sync + 'static>;
type AsyncCallback<T> = Arc<
    dyn Fn(Vec<T>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub trait CallbackItem: Send + Sync + 'static {
    fn println_path(&self) -> String;
}

pub enum CallbackWrapper<T>
where
    T: CallbackItem,
{
    Sync(SyncCallback<T>),
    Async(AsyncCallback<T>),
}

impl<T> CallbackWrapper<T>
where
    T: CallbackItem,
{
    pub fn create_sync<F>(func: F) -> Self
    where
        F: Fn(&[T]) + Send + Sync + 'static,
    {
        CallbackWrapper::Sync(Arc::new(func))
    }

    pub fn create_async<F, Fut>(func: F) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let wrapped_func = Self::wrap_async_fn(func);
        CallbackWrapper::Async(Arc::new(wrapped_func))
    }

    fn wrap_async_fn<F, Fut>(
        func: F,
    ) -> impl Fn(Vec<T>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
           + Send
           + Sync
           + 'static
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        move |objects: Vec<T>| {
            let future = func(objects);
            Box::pin(future)
                as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        }
    }

    pub fn map_to<U, F>(self, mapper: F) -> CallbackWrapper<U>
    where
        U: CallbackItem,
        T: From<U> + 'static,
        F: Fn(&U) -> T + Send + Sync + 'static,
    {
        match self {
            CallbackWrapper::Sync(func) => {
                let mapped_func = move |items: &[U]| {
                    #[allow(clippy::redundant_closure)]
                    let original_items: Vec<T> =
                        items.iter().map(|item| mapper(item)).collect();
                    func(&original_items)
                };
                CallbackWrapper::Sync(Arc::new(mapped_func))
            }
            CallbackWrapper::Async(func) => {
                let mapped_func = move |items: Vec<U>| {
                    let original_items: Vec<T> =
                        items.into_iter().map(|item| mapper(&item)).collect();
                    func(original_items)
                };
                CallbackWrapper::Async(Arc::new(mapped_func))
            }
        }
    }
}

type BinaryAsyncCallback = Arc<
    dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

pub enum BinaryCallbackWrapper {
    Async(BinaryAsyncCallback),
}

impl BinaryCallbackWrapper {
    pub fn create_async<F, Fut>(func: F) -> Self
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let wrapped_func = Self::wrap_async_fn(func);
        BinaryCallbackWrapper::Async(Arc::new(wrapped_func))
    }

    fn wrap_async_fn<F, Fut>(
        func: F,
    ) -> impl Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
           + Send
           + Sync
           + 'static
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        move |objects: Vec<u8>| {
            let future = func(objects);
            Box::pin(future)
                as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        }
    }

    pub fn call(&self, data: Vec<u8>) {
        match self {
            BinaryCallbackWrapper::Async(callback) => {
                callback(data);
            }
        }
    }
}
