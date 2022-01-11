use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::chunk::Chunk;

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<String>),
    Native(Rc<Native>),
    Closure(Rc<RefCell<Closure>>),
}

#[derive(Clone)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: Rc<String>,
    pub upvalues: Vec<FnUpvalue>,
}

pub type NativeFn = fn(usize, &[Value]) -> Value;

pub struct Native {
    pub arity: usize,
    pub name: Rc<String>,
    pub function: NativeFn,
}

pub struct Closure {
    pub function: Function,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

#[derive(Clone)]
pub struct FnUpvalue {
    pub index: u8,
    pub is_local: bool,
}

#[derive(Clone)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

impl Upvalue {
    pub fn new(location: usize) -> Self {
        Self {
            location,
            closed: None,
        }
    }
}

impl Closure {
    pub fn new(function: Function) -> RefCell<Closure> {
        RefCell::new(Self {
            upvalues: Vec::with_capacity(function.upvalues.len()),
            function,
        })
    }
}

impl Function {
    pub fn new(name: Rc<String>) -> Self {
        Self {
            chunk: Chunk::new(),
            arity: 0,
            upvalues: Vec::new(),
            name,
        }
    }
}

impl Default for Function {
    fn default() -> Self {
        Self::new(Rc::new(String::from("script")))
    }
}

impl Display for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn {}>", self.name)
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
            (Self::Native(a), Self::Native(b)) => a.name == b.name,
            (Self::Closure(a), Self::Closure(b)) => {
                a.borrow().function.name == b.borrow().function.name
            }
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
            Native(val) => write!(f, "{}", val),
            Closure(val) => write!(f, "{}", val.borrow().function),
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
        Value::Closure(Rc::new(Closure::new(f)))
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

impl From<Rc<RefCell<Closure>>> for Value {
    fn from(c: Rc<RefCell<Closure>>) -> Self {
        Value::Closure(c)
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
