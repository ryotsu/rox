use super::chunk::{Chunk, OpCode};
use super::compiler::Parser;
use super::value::Value;

#[cfg(feature = "debug_trace_execution")]
use super::debug;

const STACK_MAX: usize = 256;

pub struct VM {
    pub chunk: Chunk,
    ip: usize,
    stack: [Value; STACK_MAX],
    stack_top: usize,
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

macro_rules! binary_op {
    ($self:ident, $op:tt) => {
        {
            let b = $self.pop();
            let a = $self.pop();

            let value = match (a, b) {
                (Value::Number(a), Value::Number(b)) => Value::Number(a $op b),
                _ => {

                    $self.runtime_error("Operands must be numbers.");
                    return InterpretResult::RuntimeError;
                }
            };

            $self.push(value);
        }
    };
}

macro_rules! binary_cmp {
    ($self:ident, $op:tt) => {
        {
            let b = $self.pop();
            let a = $self.pop();
            $self.push(Value::Bool(a $op b));
        }
    };
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ip: 0,
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
        }
    }

    fn read_byte(&mut self) -> OpCode {
        self.ip += 1;
        self.chunk.code[self.ip - 1]
    }

    fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.chunk.constants[index]
    }

    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
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
        use Value::*;

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
                OpNil => self.push(Nil),
                OpTrue => self.push(Bool(true)),
                OpFalse => self.push(Bool(false)),
                OpEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Bool(a == b));
                }
                OpGreater => binary_cmp!(self, >),
                OpLess => binary_cmp!(self, <),
                OpAdd => binary_op!(self, +),
                OpSubtract => binary_op!(self, -),
                OpMultiply => binary_op!(self, *),
                OpDivide => binary_op!(self, /),
                OpNot => {
                    let value = self.pop().is_falsey();
                    self.push(Bool(!value))
                }
                OpNegate => {
                    if let Number(value) = self.pop() {
                        self.push(Number(-value))
                    } else {
                        self.runtime_error("Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpReturn => {
                    println!("{}", self.pop());
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
