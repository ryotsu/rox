use crate::{gc::GcRef, value::Value};

#[derive(Copy, Clone, Debug)]
pub enum OpCode {
    OpConstant(u8),
    OpNil,
    OpTrue,
    OpFalse,
    OpPop,
    OpGetLocal(u8),
    OpSetLocal(u8),
    OpGetGlobal(u8),
    OpDefineGlobal(u8),
    OpSetGlobal(u8),
    OpGetUpvalue(u8),
    OpSetUpvalue(u8),
    OpGetProperty(u8),
    OpSetProperty(u8),
    OpGetSuper(u8),
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
    OpJump(u16),
    OpJumpIfFalse(u16),
    OpLoop(u16),
    OpCall(u8),
    OpInvoke(u8, u8),
    OpSuperInvoke(u8, u8),
    OpClosure(u8),
    OpCloseUpvalue,
    OpReturn,
    OpClass(u8),
    OpInherit,
    OpMethod(u8),
}

#[derive(Debug)]
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

    pub fn write<T: Into<OpCode>>(&mut self, op_code: T, line: u32) -> usize {
        self.code.push(op_code.into());
        self.lines.push(line);
        self.code.len() - 1
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn write_constant(&mut self, value: Value, line: u32) -> usize {
        let index = self.add_constant(value);
        self.write(OpCode::OpConstant(index as u8), line);
        index
    }

    pub fn read_constant(&self, index: u8) -> Value {
        self.constants[index as usize]
    }

    pub fn read_string(&self, index: u8) -> GcRef<String> {
        if let Value::String(s) = self.read_constant(index) {
            s
        } else {
            panic!("Constant is not String");
        }
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
