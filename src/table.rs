use crate::{gc::GcRef, value::Value};

pub type Table = std::collections::HashMap<GcRef<String>, Value>;
