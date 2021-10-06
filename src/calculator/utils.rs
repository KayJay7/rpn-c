use super::execution::Object;
use super::Token;
use num_traits::{One, Zero};
use ramp::rational::Rational;
use ramp::Int;
use std::collections::HashMap;
use Object::*;
use Token::*;

pub enum Found {
    NotFound,
    FoundAt(usize),
}

#[inline]
pub fn clip_head(stack: &mut Vec<Token>, table: &HashMap<String, Object>) -> Vec<Token> {
    let mut to_copy = 1;
    let mut i = stack.len();

    // Counts arguments until it reaches 0 or the stack ends
    while to_copy > 0 && i > 0 {
        match &stack[i - 1] {
            Identifier(name) => {
                // Check table
                match table.get(name) {
                    Some(Function(arity, _)) | Some(Iterative(arity, _, _, _)) => {
                        to_copy += arity;
                        to_copy -= 1;
                    }
                    _ => to_copy -= 1,
                }
            }

            Number(_) => to_copy -= 1,

            Argument(_) => {
                eprintln!("Arguments are only allowed in functions");
                i = 1;
            }

            Plus | Minus | Times | Divide | PositiveMinus | IntegerDiv | Exp => to_copy += 1,

            If | ExpMod => to_copy += 2,

            _ => panic!("Corrupted stack"),
        }

        // Moves index
        i -= 1;
    }

    if to_copy == 0 {
        // If it made it to the end, split on i
        stack.split_off(i)
    } else {
        // otherwise returns an empty stack
        Vec::new()
    }
}

#[inline]
pub fn floor_abs(x: Rational, role: &'static str, position: &'static str) -> Int {
    if !x.ge(&Rational::zero()) {
        eprintln!("{} was not positive in {}", role, position);
    }
    let (num, den) = x.into_parts();
    if !den.is_one() {
        eprintln!("{} was not an integer in {}", role, position);
    }

    (num / den).abs()
}
