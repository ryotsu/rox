use crate::chunk::OpCode;
use crate::compiler::compile;
use crate::native::*;
use crate::table::Table;
use crate::value::{Class, Closure, Instance, Native, NativeFn, Upvalue, Value};

use std::cell::{Ref, RefCell};
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::rc::Rc;

#[cfg(feature = "debug_trace_execution")]
use crate::debug;

const FRAME_MAX: usize = 64;
const STACK_MAX: usize = FRAME_MAX * 256;

pub struct VM {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: Table,
    open_upvalues: VecDeque<Rc<RefCell<Upvalue>>>,
}

struct CallFrame {
    closure: Rc<RefCell<Closure>>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    fn new(closure: Rc<RefCell<Closure>>, slot: usize) -> Self {
        Self {
            closure,
            ip: 0,
            slot,
        }
    }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

macro_rules! binary_op {
    ($self:ident, +) => {{
        let b = $self.pop();
        let a = $self.pop();

        let value = match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a + b).into(),
            (Value::String(a), Value::String(b)) => {
                (String::with_capacity(a.len() + b.len()) + &a + &b).into()
            }
            _ => {
                $self.runtime_error("Both operands must be numbers/strings.");
                return InterpretResult::RuntimeError;
            }
        };

        $self.push(value);
    }};
    ($self:ident, $op:tt) => {{
        let b = $self.pop();
        let a = $self.pop();

        let value = match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a $op b).into(),
            _ => {
                $self.runtime_error("Operands must be numbers.");
                return InterpretResult::RuntimeError;
            }
        };

        $self.push(value);
    }};
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: Vec::with_capacity(FRAME_MAX),
            stack: Vec::with_capacity(STACK_MAX),
            globals: Table::new(),
            open_upvalues: VecDeque::new(),
        };

        vm.define_native("clock", 0, clock_native);
        vm
    }

    fn read_byte(&mut self) -> OpCode {
        self.current_frame_mut().ip += 1;
        self.current_closure().function.chunk.code[self.current_frame().ip - 1]
    }

    fn read_short(&mut self) -> usize {
        self.current_frame_mut().ip += 2;
        (self.current_closure().function.chunk.code[self.current_frame().ip - 2] as usize) << 8
            | self.current_closure().function.chunk.code[self.current_frame().ip - 1] as usize
    }

    fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.current_closure().function.chunk.constants[index].clone()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - distance - 1]
    }

    fn call(&mut self, closure: Rc<RefCell<Closure>>, arg_count: usize) -> bool {
        if arg_count != closure.borrow().function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                closure.borrow().function.arity,
                arg_count
            ));
            return false;
        }

        if self.frames.len() == FRAME_MAX {
            self.runtime_error("Stack overflow.");
            return false;
        }

        let frame = CallFrame::new(closure, self.stack.len() - arg_count - 1);
        self.frames.push(frame);

        true
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> bool {
        match callee {
            Value::Class(class) => {
                let slot = self.stack.len() - arg_count - 1;
                self.stack[slot] = Instance::new(class).into();
                true
            }
            Value::Closure(closure) => self.call(closure, arg_count),
            Value::Native(function) => {
                let offset = self.stack.len() - arg_count;
                let value = (function.function)(arg_count, &self.stack[offset..]);
                self.stack.truncate(offset - 1);
                self.push(value);
                true
            }
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    fn capture_upvalue(&mut self, location: usize) -> Rc<RefCell<Upvalue>> {
        for upvalue in &self.open_upvalues {
            if upvalue.borrow().location == location {
                return upvalue.clone();
            }
        }

        let upvalue = Upvalue::new(location);
        let upvalue = Rc::new(RefCell::new(upvalue));

        self.open_upvalues.push_back(upvalue.clone());
        upvalue
    }

    fn close_upvalues(&mut self, last: usize) {
        while self
            .open_upvalues
            .front()
            .map_or(false, |uv| uv.borrow().location >= last)
        {
            let upvalue = self.open_upvalues.pop_front().unwrap();
            let location = upvalue.borrow().location;
            upvalue.borrow_mut().closed = Some(self.stack[location].clone());
        }
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);

        for frame in self.frames.iter().rev() {
            let function = &frame.closure.borrow().function;
            let index = frame.ip - 1;
            eprint!("[line {}] in ", function.chunk.lines[index]);
            if function.name.as_str() == "" {
                eprintln!("script");
            } else {
                eprintln!("{}", function.name);
            }
        }

        self.reset_stack();
    }

    fn define_native(&mut self, name: &str, arity: usize, native: NativeFn) {
        let function = Native {
            name: Rc::new(name.to_string()),
            arity,
            function: native,
        };

        self.globals.insert(name.into(), function.into());
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn current_closure(&self) -> Ref<Closure> {
        self.current_frame().closure.borrow()
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let function = compile(source);
        if function.is_none() {
            return InterpretResult::CompileError;
        }
        let function = function.unwrap();

        let closure = Rc::new(Closure::new(function));
        self.push(closure.clone().into());
        self.frames.push(CallFrame::new(closure, 0));

        self.run()
    }

    fn run(&mut self) -> InterpretResult {
        use OpCode::*;

        loop {
            #[cfg(feature = "debug_trace_execution")]
            {
                print!("          ");
                for value in &self.stack {
                    print!("[ {} ]", value)
                }
                println!();

                let ip = self.current_frame().ip;
                debug::disassemble_instruction(&self.current_closure().function.chunk, ip);
            }

            let instruction = self.read_byte();
            match instruction {
                OpConstant => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                OpNil => self.push(Value::Nil),
                OpTrue => self.push(true.into()),
                OpFalse => self.push(false.into()),
                OpPop => {
                    self.pop();
                }
                OpGetLocal => {
                    let slot = self.read_byte();
                    let value = self.stack[self.current_frame().slot + slot as usize].clone();
                    self.push(value);
                }
                OpSetLocal => {
                    let slot = self.read_byte();
                    let index = self.current_frame().slot + slot as usize;
                    self.stack[index] = self.peek(0).clone();
                }
                OpGetGlobal => {
                    let name: String = self.read_constant().into();
                    let value = match self.globals.get(&name) {
                        Some(value) => value.clone(),
                        None => {
                            self.runtime_error(&format!("Undefined variable {}", name));
                            return InterpretResult::RuntimeError;
                        }
                    };

                    self.push(value);
                }
                OpDefineGlobal => {
                    let name = self.read_constant().into();
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                OpSetGlobal => {
                    let name: String = self.read_constant().into();
                    let value = self.peek(0).clone();
                    if let Entry::Occupied(mut e) = self.globals.entry(name.clone()) {
                        e.insert(value);
                    } else {
                        self.runtime_error(&format!("Undefined variable {}", name));
                        return InterpretResult::RuntimeError;
                    }
                }
                OpGetUpvalue => {
                    let slot = self.read_byte();
                    let value = {
                        let current_closure = self.current_closure();
                        let upvalue = current_closure.upvalues[slot as usize].borrow();
                        if let Some(value) = &upvalue.closed {
                            value.clone()
                        } else {
                            self.stack[upvalue.location].clone()
                        }
                    };

                    self.push(value)
                }
                OpSetUpvalue => {
                    let slot = self.read_byte();
                    let value = self.peek(0).clone();
                    let mut change_stack = None;
                    {
                        let current_closure = self.current_closure();
                        let mut upvalue = current_closure.upvalues[slot as usize].borrow_mut();
                        if upvalue.closed.is_none() {
                            change_stack = Some((upvalue.location, value));
                        } else {
                            upvalue.closed = Some(value);
                        }
                    }

                    if let Some((location, value)) = change_stack {
                        self.stack[location] = value;
                    }
                }
                OpGetProperty => {
                    match self.peek(0) {
                        Value::Instance(_) => (),
                        _ => {
                            self.runtime_error("Only instances have properties.");
                            return InterpretResult::RuntimeError;
                        }
                    }

                    let instance: Rc<Instance> = self.peek(0).clone().into();
                    let name: String = self.read_constant().into();
                    if let Some(value) = instance.fields.borrow().get(&name) {
                        self.pop();
                        self.push(value.clone())
                    } else {
                        self.runtime_error(&format!("Undefined property '{}'.", name));
                        return InterpretResult::RuntimeError;
                    };
                }

                OpSetProperty => {
                    match self.peek(1) {
                        Value::Instance(_) => (),
                        _ => {
                            self.runtime_error("Only instances have properties.");
                            return InterpretResult::RuntimeError;
                        }
                    }

                    let instance: Rc<Instance> = self.peek(1).clone().into();
                    let name: String = self.read_constant().into();
                    let value = self.peek(0).clone();
                    instance.fields.borrow_mut().insert(name, value);
                    let value = self.pop();
                    self.pop();
                    self.push(value);
                }
                OpEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push((a == b).into());
                }
                OpGreater => binary_op!(self, >),
                OpLess => binary_op!(self, <),
                OpAdd => binary_op!(self, +),
                OpSubtract => binary_op!(self, -),
                OpMultiply => binary_op!(self, *),
                OpDivide => binary_op!(self, /),
                OpNot => {
                    let value = self.pop().is_falsey();
                    self.push((!value).into())
                }
                OpNegate => {
                    if let Value::Number(value) = self.pop() {
                        self.push((-value).into())
                    } else {
                        self.runtime_error("Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpPrint => println!("{}", self.pop()),
                OpJump => {
                    let offset = self.read_short();
                    self.current_frame_mut().ip += offset;
                }
                OpJumpIfFalse => {
                    let offset = self.read_short();
                    if self.peek(0).is_falsey() {
                        self.current_frame_mut().ip += offset;
                    }
                }
                OpLoop => {
                    let offset = self.read_short();
                    self.current_frame_mut().ip -= offset;
                }
                OpCall => {
                    let arg_count = self.read_byte();
                    let value = self.peek(arg_count as usize).clone();
                    if !self.call_value(value, arg_count as usize) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpClosure => {
                    if let Value::Closure(closure) = self.read_constant() {
                        let length = closure.borrow().function.upvalues.len();
                        for _ in 0..length {
                            let is_local = self.read_byte() as u8;
                            let index = self.read_byte() as usize;

                            let upvalue = if is_local == 1 {
                                let upvalue_index = self.current_frame().slot + index;
                                self.capture_upvalue(upvalue_index)
                            } else {
                                self.current_closure().upvalues[index].clone()
                            };

                            closure.borrow_mut().upvalues.push(upvalue);
                        }
                        self.push(closure.clone().into());
                    }
                }
                OpCloseUpvalue => {
                    let top = self.stack.len() - 1;
                    self.close_upvalues(top);
                    self.pop();
                }
                OpReturn => {
                    let value = self.pop();
                    let slot = self.current_frame().slot;
                    self.close_upvalues(slot);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.pop();
                        return InterpretResult::Ok;
                    }

                    self.stack.truncate(slot);
                    self.push(value);
                }
                OpClass => {
                    let name = self.read_constant().into();
                    let class = Class::new(name);
                    self.push(class.into());
                }
            }
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
