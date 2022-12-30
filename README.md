# Rox

Lox programming language from [crafting interpreters](https://www.craftinginterpreters.com/) written in Rust.

## Build

By default it prints the debug opcodes and traces execution. You can build it without those features:
`$ cargo build --release --no-default-features`

## Run

To run a lox program:
`$ ./target/release/rox <filename>`

Or to jump into the REPL:
`$ ./target/release/rox`

## Test

You can test it using the [test suite](https://github.com/munificent/craftinginterpreters#testing-your-implementation).
