use std::any::Any;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum TableColumnValue {
    Int32Column(i32),
    Uint64Column(u64),
    Int64Column(i64),
    FloatColumn(f64),
    StringColumn(String),
    OptionalInt32Column(Option<i32>),
    OptionalUint64Column(Option<u64>),
    OptionalInt64Column(Option<i64>),
    OptionalFloatColumn(Option<f64>),
    OptionalStringColumn(Option<String>),
}

pub trait TableColumn: Debug {
    fn len(&self) -> usize;
    fn append(&mut self, value: TableColumnValue) -> Result<(), String>;
    fn as_any(&self) -> &dyn Any;
}

macro_rules! create_column_types {
    ($TypeName:ident, $OptionalTypeName:ident, $ValueType:ty) => {
        #[derive(Debug, Clone)]
        pub struct $TypeName(pub Vec<$ValueType>);

        #[derive(Debug, Clone)]
        pub struct $OptionalTypeName(pub Vec<Option<$ValueType>>);

        impl $TypeName {
            #[allow(dead_code)]
            pub fn values(&self) -> &[$ValueType] {
                &self.0
            }
        }

        impl $OptionalTypeName {
            #[allow(dead_code)]
            pub fn values(&self) -> &[Option<$ValueType>] {
                &self.0
            }
        }

        impl TableColumn for $TypeName {
            fn len(&self) -> usize {
                self.0.len()
            }

            fn append(
                &mut self,
                value: TableColumnValue,
            ) -> Result<(), String> {
                if let TableColumnValue::$TypeName(val) = value {
                    self.0.push(val);
                    Ok(())
                } else {
                    Err(format!("Type mismatch for {:?}", value))
                }
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl TableColumn for $OptionalTypeName {
            fn len(&self) -> usize {
                self.0.len()
            }

            fn append(
                &mut self,
                value: TableColumnValue,
            ) -> Result<(), String> {
                if let TableColumnValue::$OptionalTypeName(val) = value {
                    self.0.push(val);
                    Ok(())
                } else {
                    Err(format!("Type mismatch for {:?}", value))
                }
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

create_column_types!(Int32Column, OptionalInt32Column, i32);
create_column_types!(Uint64Column, OptionalUint64Column, u64);
create_column_types!(Int64Column, OptionalInt64Column, i64);
create_column_types!(FloatColumn, OptionalFloatColumn, f64);
create_column_types!(StringColumn, OptionalStringColumn, String);

impl TableColumnValue {
    pub fn to_string(&self) -> String {
        // Use a generic pattern for Optional variants to return "NULL" for None values.
        match self {
            TableColumnValue::Int32Column(val) => val.to_string(),
            TableColumnValue::Uint64Column(val) => val.to_string(),
            TableColumnValue::FloatColumn(val) => val.to_string(),
            TableColumnValue::StringColumn(val) => val.clone(),
            // Handle optional types using a pattern that matches any Some variant and calls to_string on its content.
            // For None, return "NULL".
            TableColumnValue::OptionalInt32Column(Some(val)) => val.to_string(),
            TableColumnValue::OptionalUint64Column(Some(val)) => {
                val.to_string()
            }
            TableColumnValue::OptionalInt64Column(Some(val)) => val.to_string(),
            TableColumnValue::OptionalFloatColumn(Some(val)) => val.to_string(),
            TableColumnValue::OptionalStringColumn(Some(val)) => val.clone(),
            // Match any None variant for Optional types
            _ => {
                log::error!("Unexpected TableColumnValue: {:?}", self);
                "NULL".to_string()
            }
        }
    }
}
