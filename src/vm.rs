use super::chunk::{Chunk, OpCode};
use super::compiler::Parser;
use super::table::Table;
use super::value::Value;

use std::collections::hash_map::Entry;
use std::mem;

#[cfg(feature = "debug_trace_execution")]
use super::debug;

const STACK_MAX: usize = 256;

pub struct VM {
    pub chunk: Chunk,
    ip: usize,
    stack: [Value; STACK_MAX],
    stack_top: usize,
    globals: Table,
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

macro_rules! binary_op {
    ($self:ident, +) => {{
        let b = $self.pop();
        let a = $self.pop();

        let value = match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a + b).into(),
            (Value::String(a), Value::String(b)) => {
                (String::with_capacity(a.len() + b.len()) + &a + &b).into()
            }
            _ => {
                $self.runtime_error("Addition not supported on non number/string operands.");
                return InterpretResult::RuntimeError;
            }
        };

        $self.push(value);
    }};
    ($self:ident, $op:tt) => {{
        let b = $self.pop();
        let a = $self.pop();

        let value = match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a $op b).into(),
            _ => {
                $self.runtime_error("Operation not supported on non number operands.");
                return InterpretResult::RuntimeError;
            }
        };

        $self.push(value);
    }};
}

macro_rules! binary_cmp {
    ($self:ident, $op:tt) => {
        {
            let b = $self.pop();
            let a = $self.pop();
            $self.push((a $op b).into());
        }
    };
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ip: 0,
            stack: unsafe { mem::zeroed() },
            stack_top: 0,
            globals: Table::new(),
        }
    }

    fn read_byte(&mut self) -> OpCode {
        self.ip += 1;
        self.chunk.code[self.ip - 1]
    }

    fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        mem::take(&mut self.chunk.constants[index])
    }

    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        mem::take(&mut self.stack[self.stack_top])
    }

    fn peek(&mut self, distance: usize) -> &Value {
        &self.stack[self.stack_top - 1 - distance]
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);

        eprintln!("[line {}] in script", self.chunk.lines[self.ip - 1]);
        self.reset_stack();
    }

    pub fn interpret(&mut self, source: &str) {
        let mut chunk = Chunk::new();
        Parser::compile(source, &mut chunk);
        self.chunk = chunk;

        self.ip = 0;
        self.run();
    }

    fn run(&mut self) -> InterpretResult {
        use OpCode::*;

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                print!("          ");
                for index in 0..self.stack_top {
                    print!("[ {} ]", self.stack[index])
                }
                println!();

                debug::disassemble_instruction(&self.chunk, self.ip);
            }

            let instruction = self.read_byte();
            match instruction {
                OpConstant => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                OpNil => self.push(Value::Nil),
                OpTrue => self.push(true.into()),
                OpFalse => self.push(false.into()),
                OpPop => {
                    self.pop();
                }
                OpGetLocal => {
                    let slot = self.read_byte();
                    self.push(self.stack[slot as usize].clone());
                }
                OpSetLocal => {
                    let slot = self.read_byte();
                    self.stack[slot as usize] = self.peek(0).clone();
                }
                OpGetGlobal => {
                    let name: String = self.read_constant().into();
                    let value = match self.globals.get(&name) {
                        Some(value) => value.clone(),
                        None => {
                            self.runtime_error(&format!("Undefined variable {}", name));
                            return InterpretResult::RuntimeError;
                        }
                    };

                    self.push(value);
                }
                OpDefineGlobal => {
                    let name = self.read_constant().into();
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                OpSetGlobal => {
                    let name: String = self.read_constant().into();
                    let value = self.peek(0).clone();
                    if let Entry::Occupied(mut e) = self.globals.entry(name.clone()) {
                        e.insert(value);
                    } else {
                        self.runtime_error(&format!("Undefined variable {}", name));
                        return InterpretResult::RuntimeError;
                    }
                }
                OpEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push((a == b).into());
                }
                OpGreater => binary_cmp!(self, >),
                OpLess => binary_cmp!(self, <),
                OpAdd => binary_op!(self, +),
                OpSubtract => binary_op!(self, -),
                OpMultiply => binary_op!(self, *),
                OpDivide => binary_op!(self, /),
                OpNot => {
                    let value = self.pop().is_falsey();
                    self.push((!value).into())
                }
                OpNegate => {
                    if let Value::Number(value) = self.pop() {
                        self.push((-value).into())
                    } else {
                        self.runtime_error("Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpPrint => println!("{}", self.pop()),
                OpReturn => {
                    //println!("{}", self.pop());
                    return InterpretResult::Ok;
                }
            }
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
