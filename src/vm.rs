use super::chunk::{Chunk, OpCode};
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

impl VM {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ip: 0,
            stack: [0.; STACK_MAX],
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

    fn get_operands(&mut self) -> (Value, Value) {
        let b = self.pop();
        let a = self.pop();
        (a, b)
    }

    fn add(&mut self) {
        let (a, b) = self.get_operands();
        self.push(a + b);
    }

    fn subtract(&mut self) {
        let (a, b) = self.get_operands();
        self.push(a - b);
    }

    fn multiply(&mut self) {
        let (a, b) = self.get_operands();
        self.push(a * b);
    }

    fn divide(&mut self) {
        let (a, b) = self.get_operands();
        self.push(a / b);
    }

    pub fn interpret(&mut self, chunk: Chunk) -> InterpretResult {
        self.chunk = chunk;
        self.ip = 0;
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

                debug::disassemble_instruction(&self.chunk, self.ip);
            }

            let instruction = self.read_byte();
            match instruction {
                OpConstant => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                OpAdd => self.add(),
                OpSubtract => self.subtract(),
                OpMultiply => self.multiply(),
                OpDivide => self.divide(),
                OpNegate => {
                    let value = self.pop();
                    self.push(-value);
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
