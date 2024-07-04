// add debug
pub trait ServerSpecTrait {
    fn name(&self) -> &str;
}

macro_rules! define_and_impl_server_spec {
    ($type:ident) => {
        struct $type {
            name: String,
        }

        impl ServerSpecTrait for $type {
            fn name(&self) -> &str {
                &self.name
            }
        }
    };
}
