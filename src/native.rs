use crate::value::Value;

pub fn clock_native(_arg_count: usize, _values: &[Value]) -> Value {
    //Native clock doesn't work
    0_f64.into()
}
