use super::debug::disassemble_chunk;
use super::value::Value;

use std::mem;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum OpCode {
    OpConstant,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNegate,
    OpReturn,
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        unsafe { mem::transmute(value) }
    }
}

pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write<T: Into<OpCode>>(&mut self, op_code: T, line: u32) {
        self.code.push(op_code.into());
        self.lines.push(line);
    }

    pub fn write_constant(&mut self, value: Value, line: u32) -> u8 {
        self.constants.push(value);
        self.write(OpCode::OpConstant, line);
        let index = self.constants.len() as u8 - 1;
        self.write(index, line);
        index
    }

    pub fn disassemble(&self, name: &str) {
        disassemble_chunk(self, name)
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
