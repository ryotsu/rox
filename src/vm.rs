use super::chunk::{Chunk, OpCode};
use super::compiler::Parser;
use super::native::*;
use super::table::Table;
use super::value::{Function, Native, NativeFn, Value};

use std::collections::hash_map::Entry;
use std::mem;
use std::rc::Rc;

#[cfg(feature = "debug_trace_execution")]
use super::debug;

const FRAME_MAX: usize = 64;
const STACK_MAX: usize = FRAME_MAX * 256;

pub struct VM {
    frames: Vec<CallFrame>,
    stack: [Value; STACK_MAX],
    stack_top: usize,
    globals: Table,
}

struct CallFrame {
    function: Rc<Function>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    fn from(function: Rc<Function>, ip: usize, slot: usize) -> Self {
        Self { function, ip, slot }
    }
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
                $self.runtime_error("Both operands must be numbers/strings.");
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
                $self.runtime_error("Operands must be numbers.");
                return InterpretResult::RuntimeError;
            }
        };

        $self.push(value);
    }};
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: Vec::with_capacity(FRAME_MAX),
            stack: unsafe { mem::zeroed() },
            stack_top: 0,
            globals: Table::new(),
        };

        vm.define_native("clock", 0, clock_native);
        vm
    }

    fn read_byte(&mut self) -> OpCode {
        self.frame_mut().ip += 1;
        self.chunk().code[self.frame().ip - 1]
    }

    fn read_short(&mut self) -> usize {
        self.frame_mut().ip += 2;
        (self.chunk().code[self.frame().ip - 2] as usize) << 8
            | self.chunk().code[self.frame().ip - 1] as usize
    }

    fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.chunk().constants[index].clone()
    }

    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        mem::take(&mut self.stack[self.stack_top])
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack_top - 1 - distance]
    }

    fn call(&mut self, function: Rc<Function>, arg_count: u8) -> bool {
        if arg_count != function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                function.arity, arg_count
            ));
            return false;
        }

        if self.frames.len() == FRAME_MAX {
            self.runtime_error("Stack overflow.");
            return false;
        }

        let frame = CallFrame::from(function, 0, self.stack_top - arg_count as usize - 1);
        self.frames.push(frame);

        true
    }

    fn call_value(&mut self, callee: Value, arg_count: u8) -> bool {
        match callee {
            Value::Function(function) => self.call(function, arg_count),
            Value::Native(function) => {
                let value = (function.function)(
                    arg_count,
                    &self.stack[(self.stack_top - arg_count as usize)..],
                );
                self.stack_top -= arg_count as usize + 1;
                self.push(value);
                true
            }
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
        self.frames.clear();
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);

        for frame in self.frames.iter().rev() {
            let function = &frame.function;
            let index = frame.ip - 1;
            eprint!("[line {}] in ", function.chunk.lines[index]);
            if function.name.as_str() == "" {
                eprintln!("script");
            } else {
                eprintln!("{}", function.name);
            }
        }

        self.reset_stack();
    }

    fn define_native(&mut self, name: &str, arity: u8, native: NativeFn) {
        let function = Native {
            name: Rc::new(name.to_string()),
            arity,
            function: native,
        };

        self.globals.insert(name.into(), function.into());
    }

    fn frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn chunk(&self) -> &Chunk {
        &self.frame().function.chunk
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let function = Parser::compile(source);
        if function.is_none() {
            return InterpretResult::CompileError;
        }

        let function = function.unwrap();
        self.push(function.clone().into());

        let frame = CallFrame {
            function: Rc::new(function),
            ip: 0,
            slot: 0,
        };

        self.frames.push(frame);

        self.run()
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

                let ip = self.frame().ip;
                debug::disassemble_instruction(self.chunk(), ip);
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
                    let value = self.stack[self.frame().slot + slot as usize].clone();
                    self.push(value);
                }
                OpSetLocal => {
                    let slot = self.read_byte();
                    self.stack[self.frame().slot + slot as usize] = self.peek(0).clone();
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
                OpGreater => binary_op!(self, >),
                OpLess => binary_op!(self, <),
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
                OpJump => {
                    let offset = self.read_short();
                    self.frame_mut().ip += offset;
                }
                OpJumpIfFalse => {
                    let offset = self.read_short();
                    if self.peek(0).is_falsey() {
                        self.frame_mut().ip += offset;
                    }
                }
                OpLoop => {
                    let offset = self.read_short();
                    self.frame_mut().ip -= offset;
                }
                OpCall => {
                    let arg_count = self.read_byte() as u8;
                    let value = self.peek(arg_count as usize).clone();
                    if !self.call_value(value, arg_count) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpReturn => {
                    let value = self.pop();
                    let slot = self.frame().slot;

                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.pop();
                        return InterpretResult::Ok;
                    }

                    self.stack_top = slot;
                    self.push(value);
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
