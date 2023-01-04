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

#[wasm_bindgen(module = "site/src/handle_rox.js")]
extern "C" {
    pub type Handler;

    #[wasm_bindgen(method, getter)]
    fn source(this: &Handler) -> String;

    #[wasm_bindgen(method, setter)]
    fn set_output(this: &Handler, output: &str);

    #[wasm_bindgen(method, setter)]
    fn set_error(this: &Handler, error: &str);

    #[wasm_bindgen(method, setter)]
    fn set_opcode(this: &Handler, opcode: &str);

    #[wasm_bindgen(method, setter)]
    fn set_error_lines(this: &Handler, line: u32);
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

#[wasm_bindgen]
pub fn run(handler: &Handler) {
    let mut vm = vm::VM::new(handler);
    vm.interpret();
}
