use rox::chunk::{Chunk, OpCode};
use rox::vm::VM;

fn main() {
    let mut vm = VM::new();
    let mut chunk = Chunk::new();

    chunk.write_constant(1.2, 123);
    chunk.write_constant(3.4, 123);

    chunk.write(OpCode::OpAdd, 123);

    chunk.write_constant(5.6, 123);

    chunk.write(OpCode::OpDivide, 123);

    chunk.write(OpCode::OpNegate, 123);
    chunk.write(OpCode::OpReturn, 123);
    chunk.disassemble("test chunk");

    let _ = vm.interpret(chunk);
}
