use std::any::Any;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq)]
pub enum TableColumnValue {
    Uint8Column(u8),
    Int8Column(i8),
    Uint32Column(u32),
    Int32Column(i32),
    Uint64Column(u64),
    Int64Column(i64),
    FloatColumn(f64),
    StringColumn(String),
    OptionalUint8Column(Option<u8>),
    OptionalInt8Column(Option<i8>),
    OptionalUint32Column(Option<u32>),
    OptionalInt32Column(Option<i32>),
    OptionalUint64Column(Option<u64>),
    OptionalInt64Column(Option<i64>),
    OptionalFloatColumn(Option<f64>),
    OptionalStringColumn(Option<String>),
}

impl TableColumnValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            TableColumnValue::StringColumn(s) => Some(s.clone()),
            TableColumnValue::OptionalStringColumn(os) => os.clone(),
            _ => None,
        }
    }

    pub fn as_uint8(&self) -> Option<u8> {
        match self {
            TableColumnValue::Uint8Column(u) => Some(*u),
            TableColumnValue::OptionalUint8Column(ou) => *ou,
            _ => None,
        }
    }

    pub fn as_int8(&self) -> Option<i8> {
        match self {
            TableColumnValue::Int8Column(i) => Some(*i),
            TableColumnValue::OptionalInt8Column(oi) => *oi,
            _ => None,
        }
    }

    pub fn as_uint32(&self) -> Option<u32> {
        match self {
            TableColumnValue::Uint32Column(u) => Some(*u),
            TableColumnValue::OptionalUint32Column(ou) => *ou,
            _ => None,
        }
    }

    pub fn as_int32(&self) -> Option<i32> {
        match self {
            TableColumnValue::Int32Column(i) => Some(*i),
            TableColumnValue::OptionalInt32Column(oi) => *oi,
            _ => None,
        }
    }

    pub fn as_uint64(&self) -> Option<u64> {
        match self {
            TableColumnValue::Uint64Column(u) => Some(*u),
            TableColumnValue::OptionalUint64Column(ou) => *ou,
            _ => None,
        }
    }

    pub fn as_int64(&self) -> Option<i64> {
        match self {
            TableColumnValue::Int64Column(i) => Some(*i),
            TableColumnValue::OptionalInt64Column(oi) => *oi,
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            TableColumnValue::FloatColumn(f) => Some(*f),
            TableColumnValue::OptionalFloatColumn(of) => *of,
            _ => None,
        }
    }
}

pub trait TableColumn: Debug + Send + Sync {
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
                    Err(format!(
                        "Type mismatch for {}, value={:?}",
                        stringify!($TypeName),
                        value
                    ))
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
                    Err(format!(
                        "Type mismatch for {}, value={:?}",
                        stringify!($OptionalTypeName),
                        value
                    ))
                }
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

create_column_types!(Uint8Column, OptionalUint8Column, u8);
create_column_types!(Int8Column, OptionalInt8Column, i8);
create_column_types!(Uint32Column, OptionalUint32Column, u32);
create_column_types!(Int32Column, OptionalInt32Column, i32);
create_column_types!(Uint64Column, OptionalUint64Column, u64);
create_column_types!(Int64Column, OptionalInt64Column, i64);
create_column_types!(FloatColumn, OptionalFloatColumn, f64);
create_column_types!(StringColumn, OptionalStringColumn, String);

impl TableColumnValue {
    pub fn to_string(&self) -> String {
        // Use a generic pattern for Optional variants to return "NULL" for None values.
        match self {
            TableColumnValue::Uint8Column(val) => val.to_string(),
            TableColumnValue::Int8Column(val) => val.to_string(),
            TableColumnValue::Uint32Column(val) => val.to_string(),
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
