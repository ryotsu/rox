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
            OpConstant(c) => self.constant_instruction("OP_CONSTANT", offset, c),
            OpNil => self.simple_instruction("OP_NIL", offset),
            OpTrue => self.simple_instruction("OP_TRUE", offset),
            OpFalse => self.simple_instruction("OP_FALSE", offset),
            OpPop => self.simple_instruction("OP_POP", offset),
            OpGetLocal(slot) => self.byte_instruction("OP_GET_LOCAL", offset, slot),
            OpSetLocal(slot) => self.byte_instruction("OP_SET_LOCAL", offset, slot),
            OpGetGlobal(c) => self.constant_instruction("OP_GET_GLOBAL", offset, c),
            OpDefineGlobal(constant) => {
                self.constant_instruction("OP_DEFINE_GLOBAL", offset, constant)
            }
            OpSetGlobal(c) => self.constant_instruction("OP_SET_GLOBAL", offset, c),
            OpGetUpvalue(slot) => self.byte_instruction("OP_GET_UPVALUE", offset, slot),
            OpSetUpvalue(slot) => self.byte_instruction("OP_SET_UPVALUE", offset, slot),
            OpGetProperty(c) => self.constant_instruction("OP_GET_PROPERTY", offset, c),
            OpSetProperty(c) => self.constant_instruction("OP_SET_PROPERTY", offset, c),
            OpGetSuper(c) => self.constant_instruction("OP_GET_SUPER", offset, c),
            OpEqual => self.simple_instruction("OP_EQUAL", offset),
            OpGreater => self.simple_instruction("OP_GREATER", offset),
            OpLess => self.simple_instruction("OP_LESS", offset),
            OpAdd => self.simple_instruction("OP_ADD", offset),
            OpSubtract => self.simple_instruction("OP_SUBTRACT", offset),
            OpMultiply => self.simple_instruction("OP_MULTIPLY", offset),
            OpDivide => self.simple_instruction("OP_DIVIDE", offset),
            OpNot => self.simple_instruction("OP_NOT", offset),
            OpNegate => self.simple_instruction("OP_NEGATE", offset),
            OpPrint => self.simple_instruction("OP_PRINT", offset),
            OpJump(jump) => self.jump_instruction("OP_JUMP", 1, offset, jump),
            OpJumpIfFalse(jump) => self.jump_instruction("OP_JUMP_IF_FALSE", 1, offset, jump),
            OpLoop(jump) => self.jump_instruction("OP_LOOP", -1, offset, jump),
            OpCall(slot) => self.byte_instruction("OP_CALL", offset, slot),
            OpInvoke(c, args) => self.invoke_instruction("OP_INVOKE", offset, c, args),
            OpSuperInvoke(c, args) => self.invoke_instruction("OP_SUPER_INVOKE", offset, c, args),
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

                offset + 1
            }
            OpCloseUpvalue => self.simple_instruction("OP_CLOSE_UPVALUE", offset),
            OpReturn => self.simple_instruction("OP_RETURN", offset),
            OpClass(c) => self.constant_instruction("OP_CLASS", offset, c),
            OpInherit => self.simple_instruction("OP_INHERIT", offset),
            OpMethod(c) => self.constant_instruction("OP_METHOD", offset, c),
        }
    }

    fn simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize, constant: u8) -> usize {
        let value = self.chunk.constants[constant as usize];
        println!(
            "{:<16} {:4} '{}'",
            name,
            constant,
            GcTraceFormatter::new(value, self.gc)
        );
        offset + 1
    }

    fn invoke_instruction(&self, name: &str, offset: usize, constant: u8, arg_count: u8) -> usize {
        let value = self.chunk.constants[constant as usize];
        println!(
            "{:<16} ({} args) {:4} '{}'",
            name,
            arg_count,
            constant,
            GcTraceFormatter::new(value, self.gc)
        );
        offset + 1
    }

    fn byte_instruction(&self, name: &str, offset: usize, slot: u8) -> usize {
        println!("{:<16} {:4}", name, slot);
        offset + 1
    }

    fn jump_instruction(&self, name: &str, sign: isize, offset: usize, jump: u16) -> usize {
        println!(
            "{:<16} {:4} -> {}",
            name,
            offset,
            offset as isize + 1 + sign * jump as isize
        );
        offset + 1
    }
}
