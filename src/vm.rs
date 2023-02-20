use crate::chunk::{Chunk, OpCode};
use crate::compiler::compile;
use crate::gc::{Gc, GcRef, GcTrace, GcTraceFormatter};
use crate::native::*;
use crate::table::Table;
use crate::value::{BoundMethod, Class, Closure, Instance, Native, Upvalue, Value};

use std::collections::hash_map::Entry;

#[cfg(feature = "debug_trace_execution")]
use crate::debug;

const FRAME_MAX: usize = 64;
const STACK_MAX: usize = FRAME_MAX * 256;

pub struct VM {
    gc: Gc,
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: Table,
    open_upvalues: Vec<GcRef<Upvalue>>,
    init_string: GcRef<String>,
}

#[derive(Clone)]
struct CallFrame {
    closure: GcRef<Closure>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    fn new(closure: GcRef<Closure>, slot: usize) -> Self {
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
                let a = $self.gc.deref(a);
                let b = $self.gc.deref(b);
                let result = format!("{}{}", a, b);
                let result = $self.intern(result);
                Value::String(result)
            }
            _ => {
                $self.runtime_error("Operands must be two numbers or two strings.");
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
        let mut gc = Gc::new();
        let init_string = gc.intern("init".to_string());

        let mut vm = Self {
            gc,
            frames: Vec::with_capacity(FRAME_MAX),
            stack: Vec::with_capacity(STACK_MAX),
            globals: Table::new(),
            open_upvalues: Vec::new(),
            init_string,
        };

        vm.define_native("clock", 0, Native(clock_native));
        vm
    }

    fn read_byte(&mut self) -> OpCode {
        self.current_frame_mut().ip += 1;
        self.current_chunk().code[self.current_frame().ip - 1]
    }

    fn read_short(&mut self) -> usize {
        self.current_frame_mut().ip += 2;
        (self.current_chunk().code[self.current_frame().ip - 2] as usize) << 8
            | self.current_chunk().code[self.current_frame().ip - 1] as usize
    }

    fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.current_chunk().constants[index]
    }

    fn read_string(&mut self) -> GcRef<String> {
        if let Value::String(s) = self.read_constant() {
            s
        } else {
            panic!("Constant is not String");
        }
    }

    fn alloc<T: GcTrace + 'static + std::fmt::Debug>(&mut self, object: T) -> GcRef<T> {
        self.mark_and_sweep();
        self.gc.alloc(object)
    }

    fn intern(&mut self, name: String) -> GcRef<String> {
        self.mark_and_sweep();
        self.gc.intern(name)
    }

    fn mark_and_sweep(&mut self) {
        if self.gc.should_gc() {
            #[cfg(feature = "debug_log_gc")]
            println!("-- gc begin");

            self.mark_roots();
            self.gc.collect_garbage();

            #[cfg(feature = "debug_log_gc")]
            println!("-- gc end");
        }
    }

    fn mark_roots(&mut self) {
        for &value in &self.stack {
            self.gc.mark_value(value);
        }

        for frame in &self.frames {
            self.gc.mark_object(frame.closure)
        }

        for &upvalue in &self.open_upvalues {
            self.gc.mark_object(upvalue);
        }

        self.gc.mark_table(&self.globals);
        self.gc.mark_object(self.init_string);
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - distance - 1]
    }

    fn call(&mut self, closure_ref: GcRef<Closure>, arg_count: usize) -> bool {
        let closure = self.gc.deref(closure_ref);
        let function = self.gc.deref(closure.function);

        if arg_count != function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                function.arity, arg_count
            ));
            return false;
        }

        if self.frames.len() == FRAME_MAX {
            self.runtime_error("Stack overflow.");
            return false;
        }

        let frame = CallFrame::new(closure_ref, self.stack.len() - arg_count - 1);
        self.frames.push(frame);

        true
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> bool {
        match callee {
            Value::BoundMethod(bound_ref) => {
                let bound = self.gc.deref(bound_ref);
                let slot = self.stack.len() - arg_count - 1;
                self.stack[slot] = bound.receiver;
                self.call(bound.method, arg_count)
            }
            Value::Class(class) => {
                let instance = Instance::new(class);
                let instance = self.alloc(instance);
                let slot = self.stack.len() - arg_count - 1;
                self.stack[slot] = Value::Instance(instance);

                let class = self.gc.deref(class);
                if let Some(Value::Closure(init)) = class.methods.get(&self.init_string) {
                    return self.call(*init, arg_count);
                } else if arg_count != 0 {
                    self.runtime_error(&format!("Expected 0 arguments but got {}.", arg_count));
                    return false;
                }
                true
            }
            Value::Closure(closure) => self.call(closure, arg_count),
            Value::NativeFunction(function) => {
                let offset = self.stack.len() - arg_count;
                let value = function.0(arg_count, &self.stack[offset..]);
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

    fn invoke_from_class(
        &mut self,
        class: GcRef<Class>,
        name: GcRef<String>,
        arg_count: usize,
    ) -> bool {
        let class = self.gc.deref(class);
        if let Some(Value::Closure(method)) = class.methods.get(&name) {
            return self.call(*method, arg_count);
        }

        let name = self.gc.deref(name);
        self.runtime_error(&format!("Undefined property '{}'.", name));
        false
    }

    fn invoke(&mut self, name: GcRef<String>, arg_count: usize) -> bool {
        if let Value::Instance(instance) = self.peek(arg_count) {
            let instance = self.gc.deref(instance);
            if let Some(&value) = instance.fields.get(&name) {
                let slot = self.stack.len() - arg_count - 1;
                self.stack[slot] = value;
                return self.call_value(value, arg_count);
            }

            return self.invoke_from_class(instance.class, name, arg_count);
        }

        self.runtime_error("Only instances have methods.");
        false
    }

    fn bind_method(&mut self, class: GcRef<Class>, name: GcRef<String>) -> bool {
        let class = self.gc.deref(class);
        if let Some(Value::Closure(method)) = class.methods.get(&name) {
            let bound = BoundMethod::new(self.peek(0), *method);
            let bound = self.alloc(bound);
            self.pop();
            self.push(Value::BoundMethod(bound));
            return true;
        }

        let name = self.gc.deref(name);
        self.runtime_error(&format!("Undefined property '{}'.", name));
        false
    }

    fn capture_upvalue(&mut self, location: usize) -> GcRef<Upvalue> {
        for &upvalue_ref in &self.open_upvalues {
            let upvalue = self.gc.deref(upvalue_ref);
            if upvalue.location == location {
                return upvalue_ref;
            }
        }

        let upvalue = Upvalue::new(location);
        let upvalue = self.alloc(upvalue);

        self.open_upvalues.push(upvalue);
        upvalue
    }

    fn close_upvalues(&mut self, last: usize) {
        let mut i = 0;
        while i != self.open_upvalues.len() {
            let upvalue = self.open_upvalues[i];
            let upvalue = self.gc.deref_mut(upvalue);
            if upvalue.location >= last {
                self.open_upvalues.remove(i);
                let location = upvalue.location;
                upvalue.closed = Some(self.stack[location])
            } else {
                i += 1;
            }
        }
    }

    fn define_method(&mut self, name: GcRef<String>) {
        let method = self.peek(0);
        if let Value::Class(class) = self.peek(1) {
            let class = self.gc.deref_mut(class);
            class.methods.insert(name, method);
            self.pop();
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
            let closure = self.gc.deref(frame.closure);
            let function = self.gc.deref(closure.function);
            let index = frame.ip - 1;
            let name = self.gc.deref(function.name);
            eprint!("[line {}] in ", function.chunk.lines[index]);
            if name.is_empty() {
                eprintln!("script");
            } else {
                eprintln!("{}", name);
            }
        }

        self.reset_stack();
    }

    fn define_native(&mut self, name: &str, _arity: usize, native: Native) {
        let name = self.gc.intern(name.to_owned());

        // let function = Native {
        //     name: Rc::new(name.to_string()),
        //     arity,
        //     function: native,
        // };

        self.globals.insert(name, Value::NativeFunction(native));
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn current_closure(&self) -> &Closure {
        let closure = self.current_frame().closure;
        self.gc.deref(closure)
    }

    fn current_chunk(&self) -> &Chunk {
        let closure = self.current_closure();
        let function = self.gc.deref(closure.function);
        &function.chunk
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let function = compile(source, &mut self.gc);
        if function.is_none() {
            return InterpretResult::CompileError;
        }
        let function = function.unwrap();
        let closure = Closure::new(function);
        let closure = self.alloc(closure);

        self.push(Value::Closure(closure));
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
                    let value = self.stack[self.current_frame().slot + slot as usize];
                    self.push(value);
                }
                OpSetLocal => {
                    let slot = self.read_byte();
                    let index = self.current_frame().slot + slot as usize;
                    self.stack[index] = self.peek(0);
                }
                OpGetGlobal => {
                    let name = self.read_string();
                    let value = match self.globals.get(&name) {
                        Some(&value) => value,
                        None => {
                            let name = self.gc.deref(name);
                            self.runtime_error(&format!("Undefined variable '{}'.", name));
                            return InterpretResult::RuntimeError;
                        }
                    };

                    self.push(value);
                }
                OpDefineGlobal => {
                    let name = self.read_string();
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                OpSetGlobal => {
                    let name = self.read_string();
                    let value = self.peek(0);
                    if let Entry::Occupied(mut e) = self.globals.entry(name) {
                        e.insert(value);
                    } else {
                        let name = self.gc.deref(name);
                        self.runtime_error(&format!("Undefined variable '{}'.", name));
                        return InterpretResult::RuntimeError;
                    }
                }
                OpGetUpvalue => {
                    let slot = self.read_byte();
                    let value = {
                        let current_closure = self.current_closure();
                        let upvalue = current_closure.upvalues[slot as usize];
                        let upvalue = self.gc.deref(upvalue);
                        if let Some(value) = &upvalue.closed {
                            *value
                        } else {
                            self.stack[upvalue.location]
                        }
                    };

                    self.push(value)
                }
                OpSetUpvalue => {
                    let slot = self.read_byte();
                    let value = self.peek(0);
                    let mut change_stack = None;
                    {
                        let current_closure = self.current_closure();
                        let upvalue = current_closure.upvalues[slot as usize];
                        let upvalue = self.gc.deref_mut(upvalue);
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
                    if let Value::Instance(instance) = self.peek(0) {
                        let name = self.read_string();
                        let instance = self.gc.deref(instance);
                        let class = instance.class;
                        if let Some(&value) = instance.fields.get(&name) {
                            self.pop();
                            self.push(value);
                            continue;
                        }

                        if !self.bind_method(class, name) {
                            return InterpretResult::RuntimeError;
                        }
                    } else {
                        self.runtime_error("Only instances have properties.");
                        return InterpretResult::RuntimeError;
                    }
                }

                OpSetProperty => {
                    if let Value::Instance(instance) = self.peek(1) {
                        let name = self.read_string();
                        let value = self.pop();
                        let instance = self.gc.deref_mut(instance);
                        instance.fields.insert(name, value);
                        self.pop();
                        self.push(value);
                    } else {
                        self.runtime_error("Only instances have fields.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpGetSuper => {
                    let name = self.read_string();
                    if let Value::Class(superclass) = self.pop() {
                        if !self.bind_method(superclass, name) {
                            return InterpretResult::RuntimeError;
                        }
                    } else {
                        panic!("super found no class");
                    }
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
                    self.push(value.into())
                }
                OpNegate => {
                    if let Value::Number(value) = self.pop() {
                        self.push((-value).into())
                    } else {
                        self.runtime_error("Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpPrint => {
                    let value = self.pop();
                    let formatter = GcTraceFormatter::new(value, &self.gc);
                    println!("{}", formatter);
                }
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
                    let value = self.peek(arg_count as usize);
                    if !self.call_value(value, arg_count as usize) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpInvoke => {
                    let method = self.read_string();
                    let arg_count = self.read_byte() as usize;
                    if !self.invoke(method, arg_count) {
                        return InterpretResult::RuntimeError;
                    }
                    *self.current_frame_mut() = self.frames[self.frames.len() - 1].clone();
                }
                OpSuperInvoke => {
                    let method = self.read_string();
                    let arg_count = self.read_byte() as usize;

                    if let Value::Class(superclass) = self.pop() {
                        if !self.invoke_from_class(superclass, method, arg_count) {
                            return InterpretResult::RuntimeError;
                        }
                    } else {
                        panic!("super invoke with no class");
                    }

                    *self.current_frame_mut() = self.frames[self.frames.len() - 1].clone();
                }
                OpClosure => {
                    if let Value::Closure(closure) = self.read_constant() {
                        let closure = self.gc.deref(closure);
                        let function = closure.function;
                        let length = self.gc.deref(closure.function).upvalues.len();
                        let mut upvalues = vec![];
                        for _ in 0..length {
                            let is_local = self.read_byte() as u8;
                            let index = self.read_byte() as usize;

                            let upvalue = if is_local == 1 {
                                let upvalue_index = self.current_frame().slot + index;
                                self.capture_upvalue(upvalue_index)
                            } else {
                                self.current_closure().upvalues[index]
                            };
                            upvalues.push(upvalue);
                        }

                        let closure = Closure { function, upvalues };

                        let closure = self.alloc(closure);

                        self.push(Value::Closure(closure));
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
                    let name = self.read_string();
                    let class = Class::new(name);
                    let class = self.alloc(class);
                    self.push(Value::Class(class));
                }
                OpInherit => {
                    if let Value::Class(superclass) = self.peek(1) {
                        let superclass = self.gc.deref(superclass);
                        let methods = superclass.methods.clone();
                        if let Value::Class(subclass) = self.peek(0) {
                            let subclass = self.gc.deref_mut(subclass);
                            subclass.methods.extend(methods);
                            self.pop();
                        }
                    } else {
                        self.runtime_error("Superclass must be a class.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OpMethod => {
                    let name = self.read_string();
                    self.define_method(name)
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
