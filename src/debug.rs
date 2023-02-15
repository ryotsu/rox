use crate::chunk::{Chunk, OpCode};
use crate::value::Value;
use crate::Handler;

use std::cell::RefCell;
use std::rc::Rc;

pub fn disassemble_chunk(chunk: &Chunk, name: &str, handler: Rc<RefCell<Handler>>) {
    handler.borrow_mut().set_opcode(&format!("== {} ==", name));

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset, handler.clone());
    }
}

pub fn disassemble_instruction(
    chunk: &Chunk,
    mut offset: usize,
    handler: Rc<RefCell<Handler>>,
) -> usize {
    use OpCode::*;

    let mut asm = String::new();

    asm += &format!("{:04} ", offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        asm += "   | ";
    } else {
        asm += &format!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    match instruction {
        OpConstant => constant_instruction("OP_CONSTANT", chunk, offset, handler, asm),
        OpNil => simple_instruction("OP_NIL", offset, handler, asm),
        OpTrue => simple_instruction("OP_TRUE", offset, handler, asm),
        OpFalse => simple_instruction("OP_FALSE", offset, handler, asm),
        OpPop => simple_instruction("OP_POP", offset, handler, asm),
        OpGetLocal => byte_instruction("OP_GET_LOCAL", chunk, offset, handler, asm),
        OpSetLocal => byte_instruction("OP_SET_LOCAL", chunk, offset, handler, asm),
        OpGetGlobal => constant_instruction("OP_GET_GLOBAL", chunk, offset, handler, asm),
        OpDefineGlobal => constant_instruction("OP_DEFINE_GLOBAL", chunk, offset, handler, asm),
        OpSetGlobal => constant_instruction("OP_SET_GLOBAL", chunk, offset, handler, asm),
        OpGetUpvalue => byte_instruction("OP_GET_UPVALUE", chunk, offset, handler, asm),
        OpSetUpvalue => byte_instruction("OP_SET_UPVALUE", chunk, offset, handler, asm),
        OpGetProperty => constant_instruction("OP_GET_PROPERTY", chunk, offset, handler, asm),
        OpSetProperty => constant_instruction("OP_SET_PROPERTY", chunk, offset, handler, asm),
        OpGetSuper => constant_instruction("OP_GET_SUPER", chunk, offset, handler, asm),
        OpEqual => simple_instruction("OP_EQUAL", offset, handler, asm),
        OpGreater => simple_instruction("OP_GREATER", offset, handler, asm),
        OpLess => simple_instruction("OP_LESS", offset, handler, asm),
        OpAdd => simple_instruction("OP_ADD", offset, handler, asm),
        OpSubtract => simple_instruction("OP_SUBTRACT", offset, handler, asm),
        OpMultiply => simple_instruction("OP_MULTIPLY", offset, handler, asm),
        OpDivide => simple_instruction("OP_DIVIDE", offset, handler, asm),
        OpNot => simple_instruction("OP_NOT", offset, handler, asm),
        OpNegate => simple_instruction("OP_NEGATE", offset, handler, asm),
        OpPrint => simple_instruction("OP_PRINT", offset, handler, asm),
        OpJump => jump_instruction("OP_JUMP", 1, chunk, offset, handler, asm),
        OpJumpIfFalse => jump_instruction("OP_JUMP_IF_FALSE", 1, chunk, offset, handler, asm),
        OpLoop => jump_instruction("OP_LOOP", -1, chunk, offset, handler, asm),
        OpCall => byte_instruction("OP_CALL", chunk, offset, handler, asm),
        OpInvoke => invoke_instruction("OP_INVOKE", chunk, offset, handler, asm),
        OpSuperInvoke => invoke_instruction("OP_SUPER_INVOKE", chunk, offset, handler, asm),
        OpClosure => {
            offset += 2;
            let constant = chunk.code[offset - 1];

            asm += &format!("{:<16} {:4} ", "OP_CLOSURE", constant as u8);
            asm += &format!("{}", chunk.constants[constant as usize]);
            handler.borrow_mut().set_opcode(&asm);

            if let Value::Closure(closure) = &chunk.constants[constant as usize] {
                for _ in 0..closure.borrow().function.upvalues.len() {
                    let is_local = if chunk.code[offset] as u8 == 1 {
                        "local"
                    } else {
                        "upvalue"
                    };
                    let index = chunk.code[offset + 1] as usize;
                    handler.borrow_mut().set_opcode(&format!(
                        "{:04}      | {:>20}{} {}",
                        offset, " ", is_local, index
                    ));
                    offset += 2;
                }
            }
            offset
        }
        OpCloseUpvalue => simple_instruction("OP_CLOSE_UPVALUE", offset, handler, asm),
        OpReturn => simple_instruction("OP_RETURN", offset, handler, asm),
        OpClass => constant_instruction("OP_CLASS", chunk, offset, handler, asm),
        OpInherit => simple_instruction("OP_INHERIT", offset, handler, asm),
        OpMethod => constant_instruction("OP_METHOD", chunk, offset, handler, asm),
    }
}

fn simple_instruction(
    name: &str,
    offset: usize,
    handler: Rc<RefCell<Handler>>,
    mut asm: String,
) -> usize {
    asm += name;
    handler.borrow_mut().set_opcode(&asm);
    offset + 1
}

fn constant_instruction(
    name: &str,
    chunk: &Chunk,
    offset: usize,
    handler: Rc<RefCell<Handler>>,
    mut asm: String,
) -> usize {
    let constant = chunk.code[offset + 1];
    asm += &format!("{:<16} {:4} ", name, constant as u8);
    asm += &format!("'{}'", chunk.constants[constant as usize]);
    handler.borrow_mut().set_opcode(&asm);
    offset + 2
}

fn invoke_instruction(
    name: &str,
    chunk: &Chunk,
    offset: usize,
    handler: Rc<RefCell<Handler>>,
    mut asm: String,
) -> usize {
    let constant = chunk.code[offset + 1];
    let arg_count = chunk.code[offset + 2];

    asm += &format!(
        "{:<16} ({} args) {:4} '{}'",
        name, arg_count as u8, constant as u8, chunk.constants[constant as usize]
    );
    handler.borrow_mut().set_opcode(&asm);
    offset + 3
}

fn byte_instruction(
    name: &str,
    chunk: &Chunk,
    offset: usize,
    handler: Rc<RefCell<Handler>>,
    mut asm: String,
) -> usize {
    let slot = chunk.code[offset + 1];
    asm += &format!("{:<16} {:4}", name, slot as u8);
    handler.borrow_mut().set_opcode(&asm);
    offset + 2
}

fn jump_instruction(
    name: &str,
    sign: isize,
    chunk: &Chunk,
    offset: usize,
    handler: Rc<RefCell<Handler>>,
    mut asm: String,
) -> usize {
    let jump = (chunk.code[offset + 1] as isize) << 8 | chunk.code[offset + 2] as isize;

    asm += &format!(
        "{:<16} {:4} -> {}",
        name,
        offset,
        offset as isize + 3 + sign * jump
    );

    handler.borrow_mut().set_opcode(&asm);
    offset + 3
}
