mod calculator;
use crate::calculator::Calculator;
use std::io::{stdin, BufRead};

fn main() {
    let reader = stdin();
    let mut calculator = Calculator::new();

    println!(
        "Welcome to rpn-c {}\n press Ctrl-D to quit...",
        env!("CARGO_PKG_VERSION")
    );

    for line in reader.lock().lines() {
        calculator.parse(line.expect("IO Error occurred while reading from stdin"));
    }
}
