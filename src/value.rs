use std::fmt::Display;
use std::rc::Rc;

#[derive(PartialEq, PartialOrd, Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<String>),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;

        match self {
            Nil => write!(f, "nil"),
            Bool(val) => write!(f, "{}", val),
            Number(val) => write!(f, "{}", val),
            String(val) => write!(f, "{}", val),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(Rc::new(s))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(Rc::new(String::from(s)))
    }
}

impl From<Value> for String {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => s.to_string(),
            _ => unimplemented!(),
        }
    }
}
