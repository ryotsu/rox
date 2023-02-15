pub mod chunk;
mod compiler;
mod native;
mod scanner;
mod table;
mod value;
pub mod vm;

#[cfg(any(feature = "debug_print_code", feature = "debug_trace_execution"))]
mod debug;

use js_sys::{Object, Reflect, Uint32Array};
use wasm_bindgen::prelude::*;

pub struct Handler<'a> {
    source: &'a str,
    opcode: String,
    output: String,
    error_lines: Vec<u32>,
}

impl<'a> Handler<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            opcode: String::new(),
            output: String::new(),
            error_lines: Vec::new(),
        }
    }

    fn set_opcode(&mut self, opcode: &str) {
        self.opcode.push('\n');
        self.opcode.push_str(opcode);
        self.push_state("opcode", &self.opcode);
    }

    fn set_output(&mut self, output: &str) {
        self.output.push('\n');
        self.output.push_str(output);
        self.push_state("output", &self.output);
    }

    fn set_error(&mut self, output: &str) {
        self.output.push('\n');
        self.output.push_str(output);
        self.push_state("output", &self.output);
    }

    fn set_error_lines(&mut self, line: u32) {
        self.error_lines.push(line);
        self.push_error();
    }

    fn push_state(&self, key: &str, value: &str) {
        let obj = Object::new();
        Reflect::set(&obj, &key.into(), &value.into()).unwrap();
        self.push_error();
        set_state(obj);
    }

    fn push_error(&self) {
        let obj = Object::new();
        let array = Uint32Array::from(&self.error_lines[..]);
        Reflect::set(&obj, &"errors".into(), &array).unwrap();
        set_state(obj);
    }
}

#[wasm_bindgen]
pub fn run(source: &str) {
    let handler = Handler::new(source);
    let mut vm = vm::VM::new(handler);
    vm.interpret();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = setState)]
    fn set_state(object: Object);
}
