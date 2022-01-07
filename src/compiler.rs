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
        parser.expression();
        parser.consume(TokenType::Eof, "Expect end of expression");
        parser.end_compiler();
    }

    fn advance(&mut self) {
        self.previous = mem::take(&mut self.current);

        while let Some(token) = self.scanner.next() {
            self.current = token;
            if self.current.ttype != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.value);
        }
    }

    fn consume(&mut self, ttype: TokenType, message: &str) {
        if self.current.ttype == ttype {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        self.compiler.chunk
    }

    fn emit_byte(&mut self, op_code: OpCode) {
        let line = self.previous.line;
        self.current_chunk().write(op_code, line)
    }

    fn emit_bytes(&mut self, op_code1: OpCode, op_code2: OpCode) {
        self.emit_byte(op_code1);
        self.emit_byte(op_code2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::OpReturn);
    }

    fn emit_constant(&mut self, value: Value) {
        let line = self.previous.line;
        if self.current_chunk().write_constant(value, line) > u8::MAX as usize {
            self.error("Too many constants in one chunk");
        }
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

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn number(&mut self) {
        let value: f64 = self.previous.value.parse().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn unary(&mut self) {
        let op = self.previous.ttype;

        self.parse_precedence(Precedence::Unary);

        match op {
            TokenType::Bang => self.emit_byte(OpCode::OpNot),
            TokenType::Minus => self.emit_byte(OpCode::OpNegate),
            _ => unreachable!(),
        }
    }

    fn binary(&mut self) {
        use OpCode::*;
        use TokenType::*;

        let op = self.previous.ttype;

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

    fn literal(&mut self) {
        match self.previous.ttype {
            TokenType::False => self.emit_byte(OpCode::OpFalse),
            TokenType::Nil => self.emit_byte(OpCode::OpNil),
            TokenType::True => self.emit_byte(OpCode::OpTrue),
            _ => unreachable!(),
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        match ParseRule::get_rule(self.previous.ttype).prefix {
            None => return self.error("Expect expression."),
            Some(rule) => rule(self),
        }

        while precedence <= ParseRule::get_rule(self.current.ttype).precedence {
            self.advance();
            let rule = ParseRule::get_rule(self.previous.ttype).infix.unwrap();
            rule(self);
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

        match token.ttype {
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

type ParseFn<'a> = fn(&mut Parser<'a>);

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

    fn get_rule(ttype: TokenType) -> Self {
        match ttype {
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
            TokenType::Identifier => Self::new(None, None, Precedence::None),
            TokenType::String => Self::new(None, None, Precedence::None),
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
