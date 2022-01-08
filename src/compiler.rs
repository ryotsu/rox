use crate::chunk::OpCode;
use crate::value::Value;

use super::chunk::Chunk;
use super::scanner::{Scanner, Token, TokenType};

#[cfg(debug_print_code)]
use super::debug;

use std::mem;

pub struct Parser<'a> {
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    compiler: Compiler<'a>,
    had_error: bool,
    panic_mode: bool,
}

struct Compiler<'a> {
    chunk: &'a mut Chunk,
}

impl<'a> Compiler<'a> {
    fn from(chunk: &'a mut Chunk) -> Self {
        Self { chunk }
    }
}

impl<'a> Parser<'a> {
    pub fn compile(source: &'a str, chunk: &'a mut Chunk) {
        let mut parser = Self {
            scanner: Scanner::from(source),
            previous: Token::default(),
            current: Token::default(),
            compiler: Compiler::from(chunk),
            had_error: false,
            panic_mode: false,
        };

        parser.advance();

        while !parser.check_advance(TokenType::Eof) {
            parser.declaration();
        }

        parser.end_compiler();
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

    fn check_advance(&mut self, kind: TokenType) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.advance();
        true
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        self.compiler.chunk
    }

    fn emit_byte<T: Into<OpCode>>(&mut self, op_code: T) {
        let line = self.previous.line;
        self.current_chunk().write(op_code, line)
    }

    fn emit_bytes<T: Into<OpCode>, U: Into<OpCode>>(&mut self, op_code1: T, op_code2: U) {
        self.emit_byte(op_code1);
        self.emit_byte(op_code2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::OpReturn);
    }

    fn emit_constant(&mut self, value: Value) {
        let index = self.make_constant(value);
        self.emit_bytes(OpCode::OpConstant, index as u8)
    }

    fn make_constant(&mut self, value: Value) -> usize {
        let index = self.current_chunk().add_constant(value);

        if index > u8::MAX as usize {
            self.error("Too many constants in one chunk");
        }

        index
    }

    fn end_compiler(&mut self) {
        self.emit_return();

        #[cfg(debug_print_code)]
        if !self.had_error {
            debug::disassemble_chunk(&self.current_chunk(), "code");
        }
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
        if self.check_advance(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OpCode::OpPrint);
    }

    fn declaration(&mut self) {
        if self.check_advance(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect a variable name.");

        if self.check_advance(TokenType::Equal) {
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

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn number(&mut self, _can_assign: bool) {
        let value: f64 = self.previous.value.parse().unwrap();
        self.emit_constant(value.into());
    }

    fn string(&mut self, _can_assign: bool) {
        self.emit_constant(self.previous.value.into());
    }

    fn named_variable(&mut self, name: Value, can_assign: bool) {
        let arg = self.identifier_constant(name);

        if can_assign && self.check_advance(TokenType::Equal) {
            self.expression();
            self.emit_bytes(OpCode::OpSetGlobal, arg);
        } else {
            self.emit_bytes(OpCode::OpGetGlobal, arg);
        }
    }

    fn variable(&mut self, can_assign: bool) {
        let previous = self.previous.value.into();
        self.named_variable(previous, can_assign)
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

        if can_assign && self.check_advance(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn define_variable(&mut self, global: u8) {
        self.emit_bytes(OpCode::OpDefineGlobal, global);
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.consume(TokenType::Identifier, message);
        let name = self.previous.value.into();
        self.identifier_constant(name)
    }

    fn identifier_constant(&mut self, name: Value) -> u8 {
        self.make_constant(name) as u8
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
            _ => eprint!(" at {}", token.value),
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
    Primary,
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
            TokenType::LeftParen => Self::new(Some(Parser::grouping), None, Precedence::None),
            TokenType::RightParen => Self::new(None, None, Precedence::None),
            TokenType::LeftBrace => Self::new(None, None, Precedence::None),
            TokenType::RightBrace => Self::new(None, None, Precedence::None),
            TokenType::Comma => Self::new(None, None, Precedence::None),
            TokenType::Dot => Self::new(None, None, Precedence::None),
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
            TokenType::And => Self::new(None, None, Precedence::And),
            TokenType::Class => Self::new(None, None, Precedence::None),
            TokenType::Else => Self::new(None, None, Precedence::None),
            TokenType::False => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::For => Self::new(None, None, Precedence::None),
            TokenType::Fun => Self::new(None, None, Precedence::None),
            TokenType::If => Self::new(None, None, Precedence::None),
            TokenType::Nil => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::Or => Self::new(None, None, Precedence::Or),
            TokenType::Print => Self::new(None, None, Precedence::None),
            TokenType::Return => Self::new(None, None, Precedence::None),
            TokenType::Super => Self::new(None, None, Precedence::None),
            TokenType::This => Self::new(None, None, Precedence::None),
            TokenType::True => Self::new(Some(Parser::literal), None, Precedence::None),
            TokenType::Var => Self::new(None, None, Precedence::None),
            TokenType::While => Self::new(None, None, Precedence::None),
            TokenType::Error => Self::new(None, None, Precedence::None),
            TokenType::Eof => Self::new(None, None, Precedence::None),
        }
    }
}
