use std::fmt::{Debug, Display};
use std::mem;

use crate::chunk::{Chunk, OpCode};
use crate::gc::{GcRef, GcTrace};
use crate::table::Table;

impl GcTrace for String {
    fn format(&self, f: &mut std::fmt::Formatter, _gc: &crate::gc::Gc) -> std::fmt::Result {
        write!(f, "{}", self)
    }

    fn size(&self) -> usize {
        mem::size_of::<String>() + self.as_bytes().len()
    }

    fn trace(&self, _gc: &mut crate::gc::Gc) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Number(f64),
    String(GcRef<String>),
    NativeFunction(Native),
    Closure(GcRef<Closure>),
    Class(GcRef<Class>),
    Instance(GcRef<Instance>),
    BoundMethod(GcRef<BoundMethod>),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl GcTrace for Value {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{}", value),
            Value::BoundMethod(value) => gc.deref(*value).format(f, gc),
            Value::Class(value) => gc.deref(*value).format(f, gc),
            Value::Closure(value) => gc.deref(*value).format(f, gc),
            Value::Instance(value) => gc.deref(*value).format(f, gc),
            Value::NativeFunction(_) => write!(f, "<native fn>"),
            Value::Nil => write!(f, "nil"),
            Value::Number(value) => write!(f, "{}", value),
            Value::String(value) => gc.deref(*value).format(f, gc),
        }
    }

    fn size(&self) -> usize {
        0
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        match self {
            Value::BoundMethod(value) => gc.mark_object(*value),
            Value::Class(value) => gc.mark_object(*value),
            Value::Closure(value) => gc.mark_object(*value),
            Value::Instance(value) => gc.mark_object(*value),
            Value::String(value) => gc.mark_object(*value),
            _ => (),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        panic!("Value should not be allocated")
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        panic!("Value should not be allocated")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FnUpvalue {
    pub index: u8,
    pub is_local: bool,
}

#[derive(Debug)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: GcRef<String>,
    pub upvalues: Vec<FnUpvalue>,
}

impl Function {
    pub fn new(name: GcRef<String>) -> Self {
        Self {
            chunk: Chunk::new(),
            arity: 0,
            upvalues: Vec::new(),
            name,
        }
    }
}

impl GcTrace for Function {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        let name = gc.deref(self.name);
        if name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", name)
        }
    }

    fn size(&self) -> usize {
        mem::size_of::<Function>()
            + self.upvalues.capacity() * mem::size_of::<FnUpvalue>()
            + self.chunk.code.capacity() * mem::size_of::<OpCode>()
            + self.chunk.constants.capacity() * mem::size_of::<Value>()
            + self.chunk.constants.capacity() * mem::size_of::<usize>()
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        gc.mark_object(self.name);
        for &constant in &self.chunk.constants {
            gc.mark_value(constant);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone, Copy)]
pub struct Native(pub fn(usize, &[Value]) -> Value);

impl PartialEq for Native {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Debug for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

#[derive(Debug)]
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

impl GcTrace for Upvalue {
    fn format(&self, f: &mut std::fmt::Formatter, _gc: &crate::gc::Gc) -> std::fmt::Result {
        write!(f, "upvalue")
    }

    fn size(&self) -> usize {
        mem::size_of::<Self>()
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        if let Some(obj) = self.closed {
            gc.mark_value(obj)
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct Closure {
    pub function: GcRef<Function>,
    pub upvalues: Vec<GcRef<Upvalue>>,
}

impl Closure {
    pub fn new(function: GcRef<Function>) -> Self {
        Self {
            upvalues: Vec::new(),
            function,
        }
    }
}

impl GcTrace for Closure {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        let function = gc.deref(self.function);
        function.format(f, gc)
    }

    fn size(&self) -> usize {
        mem::size_of::<Self>() + self.upvalues.capacity() * mem::size_of::<GcRef<Upvalue>>()
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        gc.mark_object(self.function);
        for &upvalue in &self.upvalues {
            gc.mark_object(upvalue);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct Class {
    pub name: GcRef<String>,
    pub methods: Table,
}

impl Class {
    pub fn new(name: GcRef<String>) -> Self {
        Class {
            name,
            methods: Table::new(),
        }
    }
}

impl GcTrace for Class {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        let name = gc.deref(self.name);
        write!(f, "{}", name)
    }

    fn size(&self) -> usize {
        mem::size_of::<Self>()
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        gc.mark_object(self.name);
        gc.mark_table(&self.methods);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct Instance {
    pub class: GcRef<Class>,
    pub fields: Table,
}

impl Instance {
    pub fn new(class: GcRef<Class>) -> Self {
        Self {
            class,
            fields: Table::new(),
        }
    }
}

impl GcTrace for Instance {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        let class = gc.deref(self.class);
        let name = gc.deref(class.name);
        write!(f, "{} instance", name)
    }

    fn size(&self) -> usize {
        mem::size_of::<Self>()
            + self.fields.capacity() * (mem::size_of::<GcRef<String>>() + mem::size_of::<Value>())
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        gc.mark_object(self.class);
        gc.mark_table(&self.fields);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: GcRef<Closure>,
}

impl BoundMethod {
    pub fn new(receiver: Value, method: GcRef<Closure>) -> Self {
        Self { receiver, method }
    }
}

impl GcTrace for BoundMethod {
    fn format(&self, f: &mut std::fmt::Formatter, gc: &crate::gc::Gc) -> std::fmt::Result {
        let method = gc.deref(self.method);
        method.format(f, gc)
    }

    fn size(&self) -> usize {
        mem::size_of::<Self>()
    }

    fn trace(&self, gc: &mut crate::gc::Gc) {
        gc.mark_object(self.method);
        gc.mark_value(self.receiver);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <&Value as std::fmt::Debug>::fmt(&self, f)
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
