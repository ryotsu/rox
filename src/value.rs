use std::fmt::Display;
use std::ops::{Add, Div, Mul, Sub};

#[derive(PartialEq, PartialOrd, Copy, Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
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
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}

macro_rules! impl_ops {
    ($interface:ident, $func:ident, $op:tt) => {
        impl $interface for Value {
            type Output = Value;

            fn $func(self, rhs: Value) -> Self::Output {
                use Value::*;

                match (self, rhs) {
                    (Number(a), Number(b)) => Number(a $op b),
                    _ => unreachable!("The operation on given operands is not defined."),
                }
            }
        }
    };
}

impl_ops!(Add, add, +);
impl_ops!(Sub, sub, -);
impl_ops!(Mul, mul, *);
impl_ops!(Div, div, /);
