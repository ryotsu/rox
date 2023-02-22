use crate::chunk::{Chunk, OpCode};
use crate::gc::{Gc, GcRef, GcTraceFormatter};
use crate::value::Value;

pub struct Disassembler<'a> {
    gc: &'a Gc,
    chunk: &'a Chunk,
}

impl<'a> Disassembler<'a> {
    pub fn new(gc: &'a Gc, chunk: &'a Chunk) -> Self {
        Self { gc, chunk }
    }

    pub fn disassemble_chunk(&self, name: GcRef<String>) {
        println!("== {} ==", self.gc.deref(name));

        let mut offset = 0;
        while offset < self.chunk.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        use OpCode::*;

        print!("{:04} ", offset);

        if offset > 0 && self.chunk.lines[offset] == self.chunk.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.chunk.lines[offset]);
        }

        let instruction = self.chunk.code[offset];
        match instruction {
            OpConstant(c) => self.constant_instruction("OP_CONSTANT", c),
            OpNil => self.simple_instruction("OP_NIL"),
            OpTrue => self.simple_instruction("OP_TRUE"),
            OpFalse => self.simple_instruction("OP_FALSE"),
            OpPop => self.simple_instruction("OP_POP"),
            OpGetLocal(slot) => self.byte_instruction("OP_GET_LOCAL", slot),
            OpSetLocal(slot) => self.byte_instruction("OP_SET_LOCAL", slot),
            OpGetGlobal(c) => self.constant_instruction("OP_GET_GLOBAL", c),
            OpDefineGlobal(constant) => self.constant_instruction("OP_DEFINE_GLOBAL", constant),
            OpSetGlobal(c) => self.constant_instruction("OP_SET_GLOBAL", c),
            OpGetUpvalue(slot) => self.byte_instruction("OP_GET_UPVALUE", slot),
            OpSetUpvalue(slot) => self.byte_instruction("OP_SET_UPVALUE", slot),
            OpGetProperty(c) => self.constant_instruction("OP_GET_PROPERTY", c),
            OpSetProperty(c) => self.constant_instruction("OP_SET_PROPERTY", c),
            OpGetSuper(c) => self.constant_instruction("OP_GET_SUPER", c),
            OpEqual => self.simple_instruction("OP_EQUAL"),
            OpGreater => self.simple_instruction("OP_GREATER"),
            OpLess => self.simple_instruction("OP_LESS"),
            OpAdd => self.simple_instruction("OP_ADD"),
            OpSubtract => self.simple_instruction("OP_SUBTRACT"),
            OpMultiply => self.simple_instruction("OP_MULTIPLY"),
            OpDivide => self.simple_instruction("OP_DIVIDE"),
            OpNot => self.simple_instruction("OP_NOT"),
            OpNegate => self.simple_instruction("OP_NEGATE"),
            OpPrint => self.simple_instruction("OP_PRINT"),
            OpJump(jump) => self.jump_instruction("OP_JUMP", 1, offset, jump),
            OpJumpIfFalse(jump) => self.jump_instruction("OP_JUMP_IF_FALSE", 1, offset, jump),
            OpLoop(jump) => self.jump_instruction("OP_LOOP", -1, offset, jump),
            OpCall(slot) => self.byte_instruction("OP_CALL", slot),
            OpInvoke(c, args) => self.invoke_instruction("OP_INVOKE", c, args),
            OpSuperInvoke(c, args) => self.invoke_instruction("OP_SUPER_INVOKE", c, args),
            OpClosure(constant) => {
                let value = self.chunk.constants[constant as usize];
                println!(
                    "{:<16} {:4} {}",
                    "OP_CLOSURE",
                    constant,
                    GcTraceFormatter::new(value, self.gc)
                );

                if let Value::Closure(closure) = value {
                    let closure = self.gc.deref(closure);
                    let function = self.gc.deref(closure.function);
                    for upvalue in &function.upvalues {
                        let is_local = if upvalue.is_local { "local" } else { "upvalue" };
                        println!("{:04}      | {:>20}{} {}", "", " ", is_local, upvalue.index);
                    }
                }
            }
            OpCloseUpvalue => self.simple_instruction("OP_CLOSE_UPVALUE"),
            OpReturn => self.simple_instruction("OP_RETURN"),
            OpClass(c) => self.constant_instruction("OP_CLASS", c),
            OpInherit => self.simple_instruction("OP_INHERIT"),
            OpMethod(c) => self.constant_instruction("OP_METHOD", c),
        }

        offset + 1
    }

    fn simple_instruction(&self, name: &str) {
        println!("{}", name);
    }

    fn constant_instruction(&self, name: &str, constant: u8) {
        let value = self.chunk.constants[constant as usize];
        println!(
            "{:<16} {:4} '{}'",
            name,
            constant,
            GcTraceFormatter::new(value, self.gc)
        );
    }

    fn invoke_instruction(&self, name: &str, constant: u8, arg_count: u8) {
        let value = self.chunk.constants[constant as usize];
        println!(
            "{:<16} ({} args) {:4} '{}'",
            name,
            arg_count,
            constant,
            GcTraceFormatter::new(value, self.gc)
        );
    }

    fn byte_instruction(&self, name: &str, slot: u8) {
        println!("{:<16} {:4}", name, slot);
    }

    fn jump_instruction(&self, name: &str, sign: isize, offset: usize, jump: u16) {
        println!(
            "{:<16} {:4} -> {}",
            name,
            offset,
            offset as isize + 1 + sign * jump as isize
        );
    }
}
