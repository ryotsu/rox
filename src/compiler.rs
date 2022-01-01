use super::scanner::{Scanner, TokenType};

pub fn compile(source: &str) {
    let scanner = Scanner::from(source);

    let mut line = u32::MAX;
    for token in scanner {
        if token.line != line {
            print!("{:4} ", token.line);
            line = token.line;
        } else {
            print!("   | ");
        }

        println!("{:2} '{}'", token.ttype as u8, token.value);

        if token.ttype == TokenType::Eof {
            break;
        }
    }
}
