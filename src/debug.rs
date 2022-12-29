use crate::chunk::{Chunk, OpCode};
use crate::value::Value;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, mut offset: usize) -> usize {
    use OpCode::*;

    print!("{:04} ", offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    match instruction {
        OpConstant => constant_instruction("OP_CONSTANT", chunk, offset),
        OpNil => simple_instruction("OP_NIL", offset),
        OpTrue => simple_instruction("OP_TRUE", offset),
        OpFalse => simple_instruction("OP_FALSE", offset),
        OpPop => simple_instruction("OP_POP", offset),
        OpGetLocal => byte_instruction("OP_GET_LOCAL", chunk, offset),
        OpSetLocal => byte_instruction("Op_SET_LOCAL", chunk, offset),
        OpGetGlobal => constant_instruction("OP_GET_GLOBAL", chunk, offset),
        OpDefineGlobal => constant_instruction("OP_DEFINE_GLOBAL", chunk, offset),
        OpSetGlobal => constant_instruction("OP_SET_GLOBAL", chunk, offset),
        OpGetUpvalue => byte_instruction("OP_GET_UPVALUE", chunk, offset),
        OpSetUpvalue => byte_instruction("OP_SET_UPVALUE", chunk, offset),
        OpGetProperty => constant_instruction("OP_GET_PROPERTY", chunk, offset),
        OpSetProperty => constant_instruction("OP_SET_PROPERTY", chunk, offset),
        OpGetSuper => constant_instruction("OP_GET_SUPER", chunk, offset),
        OpEqual => simple_instruction("OP_EQUAL", offset),
        OpGreater => simple_instruction("OP_GREATER", offset),
        OpLess => simple_instruction("OP_LESS", offset),
        OpAdd => simple_instruction("OP_ADD", offset),
        OpSubtract => simple_instruction("OP_SUBTRACT", offset),
        OpMultiply => simple_instruction("OP_MULTIPLY", offset),
        OpDivide => simple_instruction("OP_DIVIDE", offset),
        OpNot => simple_instruction("OP_NOT", offset),
        OpNegate => simple_instruction("OP_NEGATE", offset),
        OpPrint => simple_instruction("OP_PRINT", offset),
        OpJump => jump_instruction("OP_JUMP", 1, chunk, offset),
        OpJumpIfFalse => jump_instruction("OP_JUMP_IF_FALSE", 1, chunk, offset),
        OpLoop => jump_instruction("OP_LOOP", -1, chunk, offset),
        OpCall => byte_instruction("OP_CALL", chunk, offset),
        OpInvoke => invoke_instruction("OP_INVOKE", chunk, offset),
        OpSuperInvoke => invoke_instruction("OP_SUPER_INVOKE", chunk, offset),
        OpClosure => {
            offset += 2;
            let constant = chunk.code[offset - 1];
            print!("{:<16} {:4} ", "OP_CLOSURE", constant as u8);
            println!("{}", chunk.constants[constant as usize]);
            if let Value::Closure(closure) = &chunk.constants[constant as usize] {
                for _ in 0..closure.borrow().function.upvalues.len() {
                    let is_local = if chunk.code[offset] as u8 == 1 {
                        "local"
                    } else {
                        "upvalue"
                    };
                    let index = chunk.code[offset + 1] as usize;
                    println!("{:04}    | {:>20}  {} {}", offset, " ", is_local, index);
                    offset += 2;
                }
            }
            offset
        }
        OpCloseUpvalue => simple_instruction("OP_CLOSE_UPVALUE", offset),
        OpReturn => simple_instruction("OP_RETURN", offset),
        OpClass => constant_instruction("OP_CLASS", chunk, offset),
        OpInherit => simple_instruction("OP_INHERIT", offset),
        OpMethod => constant_instruction("OP_METHOD", chunk, offset),
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    print!("{:<16} {:4} ", name, constant as u8);
    println!("'{}'", chunk.constants[constant as usize]);
    offset + 2
}

fn invoke_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    let arg_count = chunk.code[offset + 2];
    println!(
        "{:<16} ({} args) {:4} '{}'",
        name, arg_count as u8, constant as u8, chunk.constants[constant as usize]
    );
    offset + 3
}

fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{:<16} {:4}", name, slot as u8);
    offset + 2
}

fn jump_instruction(name: &str, sign: isize, chunk: &Chunk, offset: usize) -> usize {
    let jump = (chunk.code[offset + 1] as isize) << 8 | chunk.code[offset + 2] as isize;

    println!(
        "{:<16} {:4} -> {}",
        name,
        offset,
        offset as isize + 3 + sign * jump
    );
    offset + 3
}
