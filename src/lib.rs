pub mod chunk;
mod compiler;
mod native;
mod scanner;
mod table;
mod value;
pub mod vm;

#[cfg(any(feature = "debug_print_code", feature = "debug_trace_execution"))]
mod debug;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello {}!", name));
}
