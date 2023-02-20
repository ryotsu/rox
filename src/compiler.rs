use crate::chunk::{Chunk, OpCode};
use crate::gc::{Gc, GcRef};
use crate::scanner::{Scanner, Token, TokenType};
use crate::value::{Closure, FnUpvalue, Function, Value};

use std::mem;

pub struct Parser<'a> {
    gc: &'a mut Gc,
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    compiler: Box<Compiler<'a>>,
    current_class: Option<ClassCompiler>,
    had_error: bool,
    panic_mode: bool,
    errors: Vec<&'static str>,
}

struct Compiler<'a> {
    enclosing: Option<Box<Compiler<'a>>>,
    locals: Vec<Local<'a>>,
    scope_depth: i32,
    function: Function,
    function_type: FunctionType,
}

#[derive(Clone)]
struct ClassCompiler {
    enclosing: Box<Option<ClassCompiler>>,
    has_superclass: bool,
}

struct Local<'a> {
    name: &'a str,
    depth: i32,
    is_captured: bool,
}

impl<'a> Local<'a> {
    fn new(name: &'a str, depth: i32) -> Self {
        Self {
            name,
            depth,
            is_captured: false,
        }
    }
}

impl ClassCompiler {
    fn new() -> Self {
        Self {
            enclosing: Box::new(None),
            has_superclass: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionType {
    Function,
    Method,
    Initializer,
    Script,
}

impl<'a> Compiler<'a> {
    fn new(ftype: FunctionType, name: GcRef<String>) -> Box<Self> {
        let mut compiler = Self {
            enclosing: None,
            locals: Vec::new(),
            scope_depth: 0,
            function: Function::new(name),
            function_type: ftype,
        };

        let local = if ftype != FunctionType::Function {
            Local::new("this", 0)
        } else {
            Local::new("", 0)
        };

        compiler.locals.push(local);

        Box::new(compiler)
    }

    fn resolve_local(&mut self, name: &str, errors: &mut Vec<&'static str>) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if name == local.name {
                if local.depth == -1 {
                    errors.push("Can't read local variable in its own initializer.");
                }

                return Some(i as u8);
            }
        }

        None
    }

    fn resolve_upvalue(&mut self, name: &str, errors: &mut Vec<&'static str>) -> Option<u8> {
        if let Some(enclosing) = self.enclosing.as_mut() {
            if let Some(local) = enclosing.resolve_local(name, errors) {
                enclosing.locals[local as usize].is_captured = true;
                return Some(self.add_upvalue(local, true, errors));
            }

            if let Some(upvalue) = enclosing.resolve_upvalue(name, errors) {
                return Some(self.add_upvalue(upvalue, false, errors));
            }
        }

        None
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool, errors: &mut Vec<&'static str>) -> u8 {
        for (i, upvalue) in self.function.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u8;
            }
        }

        if self.function.upvalues.len() == 256 {
            errors.push("Too many closure variables in function.");
        }

        self.function.upvalues.push(FnUpvalue { index, is_local });
        self.function.upvalues.len() as u8 - 1
    }

    fn is_local_declared(&self, name: &str) -> bool {
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                return false;
            }

            if name == local.name {
                return true;
            }
        }

        false
    }
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, gc: &'a mut Gc) -> Self {
        let function_name = gc.intern("script".to_owned());

        Self {
            gc,
            scanner: Scanner::from(source),
            previous: Token::default(),
            current: Token::default(),
            compiler: Compiler::new(FunctionType::Script, function_name),
            current_class: None,
            had_error: false,
            panic_mode: false,
            errors: Vec::new(),
        }
    }

    fn compile(mut self) -> Option<GcRef<Function>> {
        self.advance();

        while !self.matches(TokenType::Eof) {
            self.declaration();
        }

        //let function = self.pop_compiler();
        self.emit_return();
        if self.had_error {
            None
        } else {
            let function = self.gc.alloc(self.compiler.function);
            Some(function)
        }
    }

    fn push_compiler(&mut self, ftype: FunctionType) {
        let name = self.gc.intern(self.previous.value.to_owned());
        let new_compiler = Compiler::new(ftype, name);
        let old_compiler = mem::replace(&mut self.compiler, new_compiler);
        self.compiler.enclosing = Some(old_compiler);
    }

    fn pop_compiler(&mut self) -> Function {
        self.emit_return();

        let function = match self.compiler.enclosing.take() {
            Some(enclosing) => {
                let compiler = mem::replace(&mut self.compiler, enclosing);
                compiler.function
            }
            None => panic!("No enclosing compiler for script"),
        };

        #[cfg(feature = "debug_print_code")]
        if !self.had_error {
            let name = if function.name.as_str() != "" {
                function.name.as_str()
            } else {
                "<script>"
            };

            function.chunk.disassemble(name);
            println!();
        }

        function
    }

    fn advance(&mut self) {
        self.previous = mem::take(&mut self.current);

        while let Some(token) = self.scanner.next() {
            self.current = token;
            if self.current.kind != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.value);
        }
    }

    fn consume(&mut self, kind: TokenType, message: &str) {
        if self.current.kind == kind {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn check(&mut self, kind: TokenType) -> bool {
        self.current.kind == kind
    }

    fn matches(&mut self, kind: TokenType) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.advance();
        true
    }

    fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.compiler.function.chunk
    }

    fn emit_byte<T: Into<OpCode>>(&mut self, op_code: T) {
        let line = self.previous.line;
        self.chunk_mut().write(op_code, line)
    }

    fn emit_bytes<T: Into<OpCode>, U: Into<OpCode>>(&mut self, op_code1: T, op_code2: U) {
        self.emit_byte(op_code1);
        self.emit_byte(op_code2);
    }

    fn emit_jump<T: Into<OpCode>>(&mut self, op_code: T) -> usize {
        self.emit_byte(op_code);
        self.emit_byte(0xff);
        self.emit_byte(0xff);

        self.chunk_mut().code.len() - 2
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::OpLoop);

        let offset = self.chunk_mut().code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn emit_return(&mut self) {
        if self.compiler.function_type == FunctionType::Initializer {
            self.emit_bytes(OpCode::OpGetLocal, 0);
        } else {
            self.emit_byte(OpCode::OpNil);
        }

        self.emit_byte(OpCode::OpReturn);
    }

    fn emit_constant(&mut self, value: Value) {
        let index = self.make_constant(value);
        self.emit_bytes(OpCode::OpConstant, index)
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let index = self.chunk_mut().add_constant(value);

        match u8::try_from(index) {
            Ok(index) => index,
            Err(_) => {
                self.error("Too many constants in one chunk.");
                0
            }
        }
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.chunk_mut().code.len() - offset - 2;

        if jump > u16::MAX as usize {
            self.error("Too much code to jump over.");
        }

        self.chunk_mut().code[offset] = (((jump >> 8) & 0xff) as u8).into();
        self.chunk_mut().code[offset + 1] = ((jump & 0xff) as u8).into();
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OpCode::OpPop);
    }

    fn statement(&mut self) {
        if self.matches(TokenType::Print) {
            self.print_statement();
        } else if self.matches(TokenType::For) {
            self.for_statement();
        } else if self.matches(TokenType::If) {
            self.if_statement();
        } else if self.matches(TokenType::Return) {
            self.return_statement();
        } else if self.matches(TokenType::While) {
            self.while_statement();
        } else if self.matches(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OpCode::OpPrint);
    }

    fn return_statement(&mut self) {
        if let FunctionType::Script = self.compiler.function_type {
            self.error("Can't return from top-level code.");
        }

        if self.matches(TokenType::Semicolon) {
            self.emit_return();
        } else {
            if self.compiler.function_type == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }

            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_byte(OpCode::OpReturn);
        }
    }

    fn for_statement(&mut self) {
        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");

        if self.matches(TokenType::Semicolon) {
        } else if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.chunk_mut().code.len();
        let mut exit_jump = usize::MAX;
        if !self.matches(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit_byte(OpCode::OpPop);
        }

        if !self.matches(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::OpJump);
            let increment_start = self.chunk_mut().code.len();

            self.expression();
            self.emit_byte(OpCode::OpPop);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if exit_jump != usize::MAX {
            self.patch_jump(exit_jump);
            self.emit_byte(OpCode::OpPop);
        }

        self.end_scope();
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_byte(OpCode::OpPop);

        self.statement();

        let else_jump = self.emit_jump(OpCode::OpJump);

        self.patch_jump(then_jump);
        self.emit_byte(OpCode::OpPop);

        if self.matches(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.chunk_mut().code.len();

        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_byte(OpCode::OpPop);

        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(OpCode::OpPop);
    }

    fn declaration(&mut self) {
        if self.matches(TokenType::Class) {
            self.class_declaration();
        } else if self.matches(TokenType::Fun) {
            self.function_declaration();
        } else if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.matches(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::OpNil);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn function_declaration(&mut self) {
        let global = self.parse_variable("Expect function name");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn function(&mut self, ftype: FunctionType) {
        self.push_compiler(ftype);
        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenType::RightParen) {
            loop {
                if self.compiler.function.arity == 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }

                self.compiler.function.arity += 1;

                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);

                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        let function = self.pop_compiler();
        let upvalues = function.upvalues.clone();
        let function_id = self.gc.alloc(function);
        let closure = Closure::new(function_id);
        let closure_id = self.gc.alloc(closure);

        let index = self.make_constant(Value::Closure(closure_id));
        self.emit_bytes(OpCode::OpClosure, index);

        for upvalue in upvalues {
            self.emit_byte(upvalue.is_local as u8);
            self.emit_byte(upvalue.index);
        }
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let constant = self.identifier_constant(self.previous.value);
        let ftype = if self.previous.value == "init" {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };

        self.function(ftype);
        self.emit_bytes(OpCode::OpMethod, constant);
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name = self.previous.value;
        let name_const = self.identifier_constant(self.previous.value);
        self.declare_variable();

        self.emit_bytes(OpCode::OpClass, name_const);
        self.define_variable(name_const);

        let mut class_compiler = ClassCompiler::new();
        class_compiler.enclosing = Box::new(self.current_class.take());
        self.current_class = Some(class_compiler);

        if self.matches(TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);

            if class_name == self.previous.value {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local("super");
            self.define_variable(0);

            self.named_variable(class_name, false);
            self.emit_byte(OpCode::OpInherit);
            if let Some(class_compiler) = self.current_class.as_mut() {
                class_compiler.has_superclass = true;
            }
        }

        self.named_variable(class_name, false);

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.method();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        self.emit_byte(OpCode::OpPop);

        if let Some(class_compiler) = &self.current_class {
            if class_compiler.has_superclass {
                self.end_scope();
            }
        }

        if let Some(class) = self.current_class.take() {
            self.current_class = *class.enclosing
        }
    }

    fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;

        for i in (0..self.compiler.locals.len()).rev() {
            if self.compiler.locals[i].depth > self.compiler.scope_depth {
                if self.compiler.locals[i].is_captured {
                    self.emit_byte(OpCode::OpCloseUpvalue);
                } else {
                    self.emit_byte(OpCode::OpPop);
                }
                self.compiler.locals.pop();
            }
        }
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn number(&mut self, _can_assign: bool) {
        let value: f64 = self.previous.value.parse().unwrap();
        self.emit_constant(value.into());
    }

    fn string(&mut self, _can_assign: bool) {
        let s = self.gc.intern(self.previous.value.to_owned());
        self.emit_constant(Value::String(s));
    }

    fn named_variable(&mut self, name: &str, can_assign: bool) {
        let get_op;
        let set_op;

        let arg = if let Some(arg) = self.resolve_local(name) {
            get_op = OpCode::OpGetLocal;
            set_op = OpCode::OpSetLocal;
            arg
        } else if let Some(arg) = self.resolve_upvalue(name) {
            get_op = OpCode::OpGetUpvalue;
            set_op = OpCode::OpSetUpvalue;
            arg
        } else {
            get_op = OpCode::OpGetGlobal;
            set_op = OpCode::OpSetGlobal;
            self.identifier_constant(name)
        };

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op, arg);
        } else {
            self.emit_bytes(get_op, arg);
        }
    }

    fn variable(&mut self, can_assign: bool) {
        let previous = self.previous.value;
        self.named_variable(previous, can_assign)
    }

    fn super_(&mut self, _can_assign: bool) {
        if self.current_class.is_none() {
            self.error("Can't use 'super' outside of a class.");
        } else if self
            .current_class
            .as_ref()
            .map_or(false, |cc| !cc.has_superclass)
        {
            self.error("Can't use 'super' in a class with no superclass.");
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");
        let name = self.identifier_constant(self.previous.value);
        self.named_variable("this", false);
        if self.matches(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable("super", false);
            self.emit_bytes(OpCode::OpSuperInvoke, name);
            self.emit_byte(arg_count);
        } else {
            self.named_variable("super", false);
            self.emit_bytes(OpCode::OpGetSuper, name);
        }
    }

    fn this(&mut self, _can_assign: bool) {
        if self.current_class.is_none() {
            self.error("Can't use 'this' outside of a class.");
            return;
        }

        self.variable(false);
    }

    fn unary(&mut self, _can_assign: bool) {
        let op = self.previous.kind;

        self.parse_precedence(Precedence::Unary);

        match op {
            TokenType::Bang => self.emit_byte(OpCode::OpNot),
            TokenType::Minus => self.emit_byte(OpCode::OpNegate),
            _ => unreachable!(),
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        use OpCode::*;
        use TokenType::*;

        let op = self.previous.kind;

        let rule = ParseRule::get_rule(op);
        self.parse_precedence(rule.precedence + 1);

        match op {
            BangEqual => self.emit_bytes(OpEqual, OpNot),
            EqualEqual => self.emit_byte(OpEqual),
            Greater => self.emit_byte(OpGreater),
            GreaterEqual => self.emit_bytes(OpLess, OpNot),
            Less => self.emit_byte(OpLess),
            LessEqual => self.emit_bytes(OpGreater, OpNot),
            Plus => self.emit_byte(OpAdd),
            Minus => self.emit_byte(OpSubtract),
            Star => self.emit_byte(OpMultiply),
            Slash => self.emit_byte(OpDivide),
            _ => unreachable!(),
        }
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.previous.kind {
            TokenType::False => self.emit_byte(OpCode::OpFalse),
            TokenType::Nil => self.emit_byte(OpCode::OpNil),
            TokenType::True => self.emit_byte(OpCode::OpTrue),
            _ => unreachable!(),
        }
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_bytes(OpCode::OpCall, arg_count);
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name = self.identifier_constant(self.previous.value);

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_bytes(OpCode::OpSetProperty, name);
        } else if self.matches(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_bytes(OpCode::OpInvoke, name);
            self.emit_byte(arg_count);
        } else {
            self.emit_bytes(OpCode::OpGetProperty, name);
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        let prefix_rule = match ParseRule::get_rule(self.previous.kind).prefix {
            None => return self.error("Expect expression."),
            Some(rule) => rule,
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while precedence <= ParseRule::get_rule(self.current.kind).precedence {
            self.advance();
            let infix_rule = ParseRule::get_rule(self.previous.kind).infix.unwrap();
            infix_rule(self, can_assign);
        }

        if can_assign && self.matches(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn define_variable(&mut self, global: u8) {
        if self.compiler.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(OpCode::OpDefineGlobal, global);
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == u8::MAX {
                    self.error("Can't have more than 255 arguments.");
                }
                arg_count += 1;
                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn and(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::OpJumpIfFalse);

        self.emit_byte(OpCode::OpPop);
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump);
    }

    fn or(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        let end_jump = self.emit_jump(OpCode::OpJump);

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::OpPop);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.consume(TokenType::Identifier, message);

        self.declare_variable();
        if self.compiler.scope_depth > 0 {
            return 0;
        }

        let name = self.previous.value;
        self.identifier_constant(name)
    }

    fn mark_initialized(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }

        self.compiler.locals.last_mut().unwrap().depth = self.compiler.scope_depth;
    }

    fn identifier_constant(&mut self, name: &str) -> u8 {
        let identifier = self.gc.intern(name.to_owned());
        self.make_constant(Value::String(identifier))
    }

    fn resolve_local(&mut self, name: &str) -> Option<u8> {
        let result = self.compiler.resolve_local(name, &mut self.errors);
        while let Some(e) = self.errors.pop() {
            self.error(e);
        }

        result
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        let result = self.compiler.resolve_upvalue(name, &mut self.errors);
        while let Some(e) = self.errors.pop() {
            self.error(e);
        }

        result
    }

    fn add_local(&mut self, name: &'a str) {
        if self.compiler.locals.len() == u8::MAX as usize + 1 {
            self.error("Too many local variables in function.");
            return;
        }

        let local = Local::new(name, -1);
        self.compiler.locals.push(local);
    }

    fn declare_variable(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }

        let name = self.previous.value;

        if self.compiler.is_local_declared(name) {
            self.error("Already a variable with this name in this scope.");
        }

        self.add_local(name);
    }

    fn synchronize(&mut self) {
        use TokenType::*;

        self.panic_mode = false;

        while self.current.kind != Eof {
            if self.previous.kind == Semicolon {
                return;
            }

            match self.current.kind {
                Class | Fun | Var | For | If | While | Print | Return => return,
                _ => (),
            }

            self.advance();
        }
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.current.clone(), message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&self.previous.clone(), message);
    }

    fn error_at(&mut self, token: &Token<'a>, message: &str) {
        if self.panic_mode {
            return;
        }

        self.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        match token.kind {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => (),
            TokenType::String => eprint!(" at '\"{}\"'", token.value),
            _ => eprint!(" at '{}'", token.value),
        }

        eprintln!(": {}", message);
        self.had_error = true;
    }
}

#[repr(u8)]
#[derive(PartialOrd, PartialEq, Debug)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
}

impl std::ops::Add<u8> for Precedence {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        unsafe { mem::transmute((self as u8 + rhs) % 11) }
    }
}

type ParseFn<'a> = fn(&mut Parser<'a>, bool);

struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Precedence,
}

impl<'a> ParseRule<'a> {
    fn new(
        prefix: Option<ParseFn<'a>>,
        infix: Option<ParseFn<'a>>,
        precedence: Precedence,
    ) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }

    fn get_rule(kind: TokenType) -> Self {
        match kind {
            TokenType::LeftParen => {
                Self::new(Some(Parser::grouping), Some(Parser::call), Precedence::Call)
            }
            TokenType::RightParen => Self::new(None, None, Precedence::None),
            TokenType::LeftBrace => Self::new(None, None, Precedence::None),
            TokenType::RightBrace => Self::new(None, None, Precedence::None),
            TokenType::Comma => Self::new(None, None, Precedence::None),
            TokenType::Dot => Self::new(None, Some(Parser::dot), Precedence::Call),
            TokenType::Minus => {
                Self::new(Some(Parser::unary), Some(Parser::binary), Precedence::Term)
            }
            TokenType::Plus => Self::new(None, Some(Parser::binary), Precedence::Term),
            TokenType::Semicolon => Self::new(None, None, Precedence::None),
            TokenType::Slash => Self::new(None, Some(Parser::binary), Precedence::Factor),
            TokenType::Star => Self::new(None, Some(Parser::binary), Precedence::Factor),
            TokenType::Bang => Self::new(Some(Parser::unary), None, Precedence::None),
            TokenType::BangEqual => Self::new(None, Some(Parser::binary), Precedence::Equality),
            TokenType::Equal => Self::new(None, None, Precedence::None),
            TokenType::EqualEqual => Self::new(None, Some(Parser::binary), Precedence::Equality),
            TokenType::Greater => Self::new(None, Some(Parser::binary), Precedence::Comparison),
            TokenType::GreaterEqual => {
                Self::new(None, Some(Parser::binary), Precedence::Comparison)
            }
            TokenType::Less => Self::new(None, Some(Parser::binary), Precedence::Comparison),
            TokenType::LessEqual => Self::new(None, Some(Parser::binary), Precedence::Comparison),
            TokenType::Identifier => Self::new(Some(Parser::variable), None, Precedence::None),
            TokenType::String => Self::new(Some(Parser::string), None, Precedence::None),
            TokenType::Number => Self::new(Some(Parser::number), None, Precedence::None),
            TokenType::And => Self::new(None, Some(Parser::and), Precedence::And),
            TokenType::Class => Self::new(None, None, Precedence::None),
            TokenType::Else => Self::new(None, None, Precedence::None),
            TokenType::False => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::For => Self::new(None, None, Precedence::None),
            TokenType::Fun => Self::new(None, None, Precedence::None),
            TokenType::If => Self::new(None, None, Precedence::None),
            TokenType::Nil => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::Or => Self::new(None, Some(Parser::or), Precedence::Or),
            TokenType::Print => Self::new(None, None, Precedence::None),
            TokenType::Return => Self::new(None, None, Precedence::None),
            TokenType::Super => Self::new(Some(Parser::super_), None, Precedence::None),
            TokenType::This => Self::new(Some(Parser::this), None, Precedence::None),
            TokenType::True => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::Var => Self::new(None, None, Precedence::None),
            TokenType::While => Self::new(None, None, Precedence::None),
            TokenType::Error => Self::new(None, None, Precedence::None),
            TokenType::Eof => Self::new(None, None, Precedence::None),
        }
    }
}

pub fn compile(source: &str, gc: &mut Gc) -> Option<GcRef<Function>> {
    let parser = Parser::new(source, gc);
    parser.compile()
}
