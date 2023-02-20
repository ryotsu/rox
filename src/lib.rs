pub mod chunk;
mod compiler;
mod gc;
mod native;
mod scanner;
mod table;
mod value;
pub mod vm;

#[cfg(any(feature = "debug_print_code", feature = "debug_trace_execution"))]
mod debug;
