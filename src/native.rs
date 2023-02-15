use crate::value::Value;

pub fn clock_native(_arg_coun: usize, _values: &[Value]) -> Value {
    js_sys::Date::now().into()
}
