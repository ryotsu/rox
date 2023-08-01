# Rox

Lox programming language from [Crafting Interpreters](https://www.craftinginterpreters.com/) written in Rust.

You can check out the live playground [here](https://ryotsu.github.io/rox/).

## Build

By default it prints the debug opcodes, traces execution and logs gc. You can build it without those features:

`$ cargo build --release --no-default-features`

## Run

To run a lox program:
`$ ./target/release/rox <filename>`

Or to jump into the REPL:
`$ ./target/release/rox`

## Test

You can test it using the [test suite](https://github.com/munificent/craftinginterpreters#testing-your-implementation).
