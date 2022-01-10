use std::fmt::Display;
use std::rc::Rc;

use crate::chunk::Chunk;

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<String>),
    Function(Rc<Function>),
    Native(Rc<Native>),
}

#[derive(Clone)]
pub struct Function {
    pub arity: u8,
    pub chunk: Chunk,
    pub name: Rc<String>,
}

pub type NativeFn = fn(u8, &[Value]) -> Value;

pub struct Native {
    pub arity: u8,
    pub name: Rc<String>,
    pub function: NativeFn,
}

impl Function {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            arity: 0,
            name: Rc::new(String::new()),
        }
    }
}

impl Display for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

impl Default for Function {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.as_str() != "" {
            write!(f, "<fn {}>", self.name)
        } else {
            write!(f, "<script>")
        }
    }
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Function(a), Self::Function(b)) => a.name == b.name,
            (Self::Native(a), Self::Native(b)) => a.name == b.name,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
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
            Function(val) => write!(f, "{}", val),
            Native(val) => write!(f, "{}", val),
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

impl From<Function> for Value {
    fn from(f: Function) -> Self {
        Value::Function(Rc::new(f))
    }
}

impl From<Native> for Value {
    fn from(f: Native) -> Self {
        Value::Native(Rc::new(f))
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
