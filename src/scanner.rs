use itertools::{multipeek, MultiPeek};

use std::str::Chars;

pub struct Scanner<'a> {
    text: &'a str,
    source: MultiPeek<Chars<'a>>,
    start: usize,
    current: usize,
    pub line: u32,
    is_finished: bool,
}

impl<'a> Scanner<'a> {
    pub fn from(source: &'a str) -> Self {
        Self {
            text: source,
            source: multipeek(source.chars()),
            start: 0,
            current: 0,
            line: 1,
            is_finished: false,
        }
    }

    pub fn scan_token(&mut self) -> Option<Token<'a>> {
        use TokenType::*;

        self.start = self.current;

        if let Some(ch) = self.advance() {
            match ch {
                '(' => self.make_token(LeftParen),
                ')' => self.make_token(RightParen),
                '{' => self.make_token(LeftBrace),
                '}' => self.make_token(RightBrace),
                ';' => self.make_token(Semicolon),
                ',' => self.make_token(Comma),
                '.' => self.make_token(Dot),
                '-' => self.make_token(Minus),
                '+' => self.make_token(Plus),
                '/' => self.scan_comment(),
                '*' => self.make_token(Star),
                '!' => self.match_token('=', BangEqual, Bang),
                '=' => self.match_token('=', EqualEqual, Equal),
                '<' => self.match_token('=', LessEqual, Less),
                '>' => self.match_token('=', GreaterEqual, Greater),
                ' ' | '\t' | '\r' => self.scan_token(),
                '\n' => {
                    self.line += 1;
                    self.scan_token()
                }
                '"' => self.scan_string(),
                '0'..='9' => self.scan_number(),
                ch if ch.is_ascii_alphabetic() || ch == '_' => self.scan_identifier(),
                _ => self.error_token("Unexpected character"),
            }
        } else if self.is_finished {
            None
        } else {
            self.is_finished = true;
            self.make_token(Eof)
        }
    }

    fn advance(&mut self) -> Option<char> {
        self.source.next().map(|x| {
            self.current += 1;
            x
        })
    }

    fn make_token(&self, token_type: TokenType) -> Option<Token<'a>> {
        Some(Token {
            ttype: token_type,
            value: if token_type == TokenType::String {
                &self.text[self.start + 1..self.current - 1]
            } else {
                &self.text[self.start..self.current]
            },
            line: self.line,
        })
    }

    fn match_token(&mut self, expected: char, t: TokenType, f: TokenType) -> Option<Token<'a>> {
        if self.source.peek() == Some(&expected) {
            self.advance();
            self.make_token(t)
        } else {
            self.make_token(f)
        }
    }

    fn error_token(&self, message: &'a str) -> Option<Token<'a>> {
        Some(Token {
            ttype: TokenType::Error,
            line: self.line,
            value: message,
        })
    }

    fn scan_comment(&mut self) -> Option<Token<'a>> {
        if self.source.peek() == Some(&'/') {
            while self.source.peek().map_or(false, |&ch| ch != '\n') {
                self.advance();
            }

            self.scan_token()
        } else {
            self.make_token(TokenType::Slash)
        }
    }

    fn scan_string(&mut self) -> Option<Token<'a>> {
        while self.source.peek().map_or(false, |&ch| ch != '"') {
            self.source.reset_peek();
            if self.source.peek() == Some(&'\n') {
                self.line += 1;
            }
            self.advance();
        }

        if self.source.peek() == None {
            return self.error_token("Unterminated string.");
        }

        self.advance();
        self.make_token(TokenType::String)
    }

    fn scan_number(&mut self) -> Option<Token<'a>> {
        while self.source.peek().map_or(false, |ch| ch.is_ascii_digit()) {
            self.advance();
        }

        self.source.reset_peek();

        if self.source.peek() == Some(&'.')
            && self.source.peek().map_or(false, |ch| ch.is_ascii_digit())
        {
            self.advance();
            while self.source.peek().map_or(false, |ch| ch.is_ascii_digit()) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    fn scan_identifier(&mut self) -> Option<Token<'a>> {
        while self
            .source
            .peek()
            .map_or(false, |&ch| ch.is_ascii_alphabetic() || ch == '_')
        {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenType {
        use TokenType::*;

        match &self.text[self.start..self.current] {
            "and" => And,
            "class" => Class,
            "else" => Else,
            "false" => False,
            "for" => For,
            "fun" => Fun,
            "if" => If,
            "nil" => Nil,
            "or" => Or,
            "print" => Print,
            "return" => Return,
            "super" => Super,
            "this" => This,
            "true" => True,
            "var" => Var,
            "while" => While,
            _ => Identifier,
        }
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
#[repr(C)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    Identifier,
    String,
    Number,

    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Error,
    Eof,
}

#[derive(Clone)]
pub struct Token<'a> {
    pub ttype: TokenType,
    pub value: &'a str,
    pub line: u32,
}

impl<'a> Default for Token<'a> {
    fn default() -> Self {
        Token {
            ttype: TokenType::Error,
            value: "",
            line: 0,
        }
    }
}
