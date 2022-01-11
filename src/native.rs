use std::time::{SystemTime, UNIX_EPOCH};

use crate::value::Value;

pub fn clock_native(_arg_coun: usize, _values: &[Value]) -> Value {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;

    time.into()
}
