use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::rc::Rc;

use crate::chunk::Chunk;
use crate::table::Table;

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<String>),
    Native(Rc<Native>),
    Closure(Rc<RefCell<Closure>>),
    Class(Rc<Class>),
    Instance(Rc<Instance>),
    BoundMethod(Rc<BoundMethod>),
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

pub struct Class {
    pub name: Rc<String>,
    pub methods: RefCell<Table>,
}

pub struct Instance {
    pub class: Rc<Class>,
    pub fields: RefCell<Table>,
}

pub struct BoundMethod {
    pub receiver: Value,
    pub method: Rc<RefCell<Closure>>,
}

impl Instance {
    pub fn new(class: Rc<Class>) -> Self {
        Self {
            class,
            fields: RefCell::new(Table::new()),
        }
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} instance>", self.class)
    }
}

impl Class {
    pub fn new(name: Rc<String>) -> Self {
        Class {
            name,
            methods: RefCell::new(Table::new()),
        }
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
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

impl BoundMethod {
    pub fn new(receiver: Value, method: Rc<RefCell<Closure>>) -> Self {
        Self { receiver, method }
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

impl Display for BoundMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.method.borrow().function)
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

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Value::*;

        match self {
            Nil => write!(f, "nil"),
            Bool(val) => write!(f, "{}", val),
            Number(val) => write!(f, "{}", val),
            String(val) => write!(f, "{}", val),
            Native(val) => write!(f, "{}", val),
            Closure(val) => write!(f, "{}", val.borrow().function),
            Class(val) => write!(f, "{}", val),
            Instance(val) => write!(f, "{}", val),
            BoundMethod(val) => write!(f, "{}", val),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <&Value as std::fmt::Debug>::fmt(&self, f)
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

impl From<Class> for Value {
    fn from(c: Class) -> Self {
        Value::Class(Rc::new(c))
    }
}

impl From<Instance> for Value {
    fn from(i: Instance) -> Self {
        Value::Instance(Rc::new(i))
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

impl From<Value> for Rc<Instance> {
    fn from(v: Value) -> Self {
        match v {
            Value::Instance(i) => i,
            _ => unimplemented!(),
        }
    }
}

impl From<Value> for Rc<Class> {
    fn from(v: Value) -> Self {
        match v {
            Value::Class(c) => c,
            _ => unimplemented!(),
        }
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

impl From<Value> for Rc<String> {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => s,
            _ => unimplemented!(),
        }
    }
}
