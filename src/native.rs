use std::time::{SystemTime, UNIX_EPOCH};

use crate::value::Value;

pub fn clock_native(_arg_coun: usize, _values: &[Value]) -> Value {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        .into()
}
