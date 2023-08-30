use std::any::Any;
use std::fmt::Debug;
use std::collections::HashMap;

pub trait KeyValue: Debug + AnyKeyValue {
    type ValueType;
    fn get(&self, key: &str) -> Option<&Self::ValueType>;
}

pub type StringMap = HashMap<String, String>;

impl KeyValue for StringMap {
    type ValueType = String;
    fn get(&self, key: &str) -> Option<&Self::ValueType> {
        self.get(key)
    }
}

pub type ByteMap = HashMap<String, Vec<u8>>;

impl KeyValue for ByteMap {
    type ValueType = Vec<u8>;
    fn get(&self, key: &str) -> Option<&Self::ValueType> {
        self.get(key)
    }
}

pub trait AnyKeyValue: Debug + Any {
    fn get_boxed(&self, key: &str) -> Option<Box<dyn Any>>;
    fn get_string_or_default(&self, key: &str, default: &'static str) -> String {
        if let Some(boxed_value) = self.get_boxed(key) {
            if let Ok(string_value) = boxed_value.downcast::<String>() {
                return (*string_value).clone();
            }
        }
        default.to_string()
    }
}


impl<T: KeyValue + ?Sized> AnyKeyValue for T where T::ValueType: Clone + 'static {
    fn get_boxed(&self, key: &str) -> Option<Box<dyn Any>> {
        self.get(key).map(|v| Box::new(v.clone()) as Box<dyn Any>)
    }
}
