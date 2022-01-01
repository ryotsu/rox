use super::chunk::{Chunk, OpCode};

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    use OpCode::*;

    print!("{:04} ", offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    match instruction {
        OpReturn => simple_instruction("OP_RETURN", offset),
        OpAdd => simple_instruction("OP_ADD", offset),
        OpSubtract => simple_instruction("OP_SUBTRACT", offset),
        OpMultiply => simple_instruction("OP_MULTIPLY", offset),
        OpDivide => simple_instruction("OP_DIVIDE", offset),
        OpNegate => simple_instruction("OP_NEGATE", offset),
        OpConstant => constant_instruction("OP_CONSTANT", chunk, offset),
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
