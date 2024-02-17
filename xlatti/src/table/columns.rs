use std::any::Any;
use std::fmt::Debug;


#[derive(Debug, Clone)]
pub enum TableColumnValue {
    Int32(i32),
    Uint64(u64),
    Float(f64),
    String(String),
}

impl TableColumnValue {
    pub fn to_string(&self) -> String {
        match self {
            TableColumnValue::Int32(val) => val.to_string(),
            TableColumnValue::Uint64(val) => val.to_string(),
            TableColumnValue::Float(val) => val.to_string(),
            TableColumnValue::String(val) => val.clone(),
        }
    }
}

pub trait TableColumn {
    fn len(&self) -> usize;
    fn append(&mut self, value: TableColumnValue) -> Result<(), &'static str>;
    fn as_any(&self) -> &dyn Any;
    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

#[derive(Debug, Clone)]
pub struct Int32Column(pub Vec<i32>);

impl Int32Column {
    pub fn values(&self) -> &[i32] {
        &self.0
    }
}

impl TableColumn for Int32Column {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn append(&mut self, value: TableColumnValue) -> Result<(), &'static str> {
        match value {
            TableColumnValue::Int32(val) => {
                self.0.push(val);
                Ok(())
            }
            _ => Err("Type mismatch"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub struct Uint64Column(pub Vec<u64>);

impl Uint64Column {
    pub fn values(&self) -> &[u64] {
        &self.0
    }
}

impl TableColumn for Uint64Column {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn append(&mut self, value: TableColumnValue) -> Result<(), &'static str> {
        match value {
            TableColumnValue::Uint64(val) => {
                self.0.push(val);
                Ok(())
            }
            _ => Err("Type mismatch"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct FloatColumn(pub Vec<f64>);

impl FloatColumn {
    pub fn values(&self) -> &[f64] {
        &self.0
    }
}

impl TableColumn for FloatColumn {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn append(&mut self, value: TableColumnValue) -> Result<(), &'static str> {
        match value {
            TableColumnValue::Float(val) => {
                self.0.push(val);
                Ok(())
            }
            _ => Err("Type mismatch"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct StringColumn(pub Vec<String>);

impl StringColumn {
    pub fn values(&self) -> &[String] {
        &self.0
    }
}

impl TableColumn for StringColumn {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn append(&mut self, value: TableColumnValue) -> Result<(), &'static str> {
        match value {
            TableColumnValue::String(val) => {
                self.0.push(val);
                Ok(())
            }
            _ => Err("Type mismatch"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}