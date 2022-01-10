use std::time::{SystemTime, UNIX_EPOCH};

use super::value::Value;

pub fn clock_native(_arg_coun: u8, _values: &[Value]) -> Value {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;

    time.into()
}
