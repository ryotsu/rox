use super::debug::disassemble_chunk;
use super::value::Value;

use std::mem;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum OpCode {
    OpConstant,
    OpNil,
    OpTrue,
    OpFalse,
    OpPop,
    OpGetLocal,
    OpSetLocal,
    OpGetGlobal,
    OpDefineGlobal,
    OpSetGlobal,
    OpEqual,
    OpGreater,
    OpLess,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNot,
    OpNegate,
    OpPrint,
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

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn write_constant(&mut self, value: Value, line: u32) -> usize {
        let index = self.add_constant(value);
        self.write(OpCode::OpConstant, line);
        self.write(index as u8, line);
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
