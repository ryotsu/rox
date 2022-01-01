use std::io::Write;
use std::process::exit;
use std::{env, fs, io};

use rox::vm::VM;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        repl();
    } else if args.len() == 2 {
        run_file(&args[1]);
    } else {
        eprintln!("Usage: clox [path]");
        exit(64);
    }
}

fn repl() {
    let mut vm = VM::new();

    let mut line = String::with_capacity(1024);
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        if io::stdin().read_line(&mut line).is_err() {
            println!();
            break;
        }
        vm.interpret(&line);
        line.clear()
    }
}

fn run_file(path: &str) {
    let contents = fs::read_to_string(path).expect("Could not read the file.");
    let mut vm = VM::new();
    vm.interpret(&contents);
}
