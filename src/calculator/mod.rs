use execution::*;
use logos::Logos;
use num_traits::{One, Zero};
use ramp::rational::Rational;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::string::String;
use strings::*;
use utils::*;
use Found::*;
use Object::*;
use Token::*;

mod execution;
mod strings;
mod utils;

// Readable tokens from command line
#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    Identifier(String),

    #[regex("=[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    AssignVariable(String),

    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*\\|[0-9]+", |lex| String::from(lex.slice()))]
    AssignFunction(String),

    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*@[0-9]+", |lex| String::from(lex.slice()))]
    AssignIterative(String),

    #[regex("\\$[0-9]+", |lex| {
        let mut parse = lex.slice().split('$');
        parse.next();
        parse.next().unwrap().parse()
    })]
    Argument(usize),

    #[regex("\"([^\"\\\\]|\\\\n|\\\\r|\\\\t|\\\\\\\\|\\\\\"|\\\\[0-9a-fA-F][0-9a-fA-F])*\"", |lex| from_string(lex.slice()))]
    #[regex("[\\-\\+]?[0-9]+(/[0-9]+)?", |lex| lex.slice().parse())]
    Number(Rational),

    #[regex("-")]
    Minus,

    #[regex("\\+")]
    Plus,

    #[regex("\\*")]
    Times,

    #[regex("/")]
    Divide,

    #[regex("~")]
    PositiveMinus,

    #[regex("\\\\")]
    IntegerDiv,

    #[regex("\\^")]
    Exp,

    #[regex("_")]
    ExpMod,

    #[regex("\\?")]
    If,

    #[regex("=")]
    Return,

    #[regex("#")]
    Partial,

    #[regex(":")]
    Print,

    #[regex(">")]
    Flush,

    #[regex("<")]
    Duplicate,

    #[regex("!")]
    Drop,

    #[regex("%")]
    Empty,

    #[regex("&")]
    Format,

    #[regex("\\[\\]")]
    Approx,

    #[error]
    #[regex(";.*", logos::skip)]
    #[regex(r"[ \t\n\f\r]+", logos::skip)]
    Error,
}

// Implement Display for printing
impl fmt::Display for Token {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Number(num) => {
                let (num, den) = num.to_owned().into_parts();
                if den.is_one() {
                    write!(f, "{}", num)
                } else {
                    write!(f, "{}/{}", num, den)
                }
            }
            Plus => write!(f, "+"),
            Minus => write!(f, "-"),
            Times => write!(f, "*"),
            Divide => write!(f, "/"),
            IntegerDiv => write!(f, "\\"),
            If => write!(f, "?"),
            PositiveMinus => write!(f, "~"),
            Exp => write!(f, "^"),
            ExpMod => write!(f, "_"),
            Argument(index) => write!(f, "${}", index),
            Identifier(name) => write!(f, "{}", name),
            _ => write!(f, "Unprintable"),
        }
    }
}

// Structure for keeping the current state of the calculator
pub struct Calculator {
    stack: Vec<Token>,
    table: HashMap<String, Object>,
}

impl Calculator {
    // Empty calculator
    #[inline]
    pub fn new() -> Calculator {
        Calculator {
            stack: Vec::new(),
            table: HashMap::new(),
        }
    }

    // To be called from main,
    // Parse a line into tokens and compute them
    #[inline]
    pub fn parse(&mut self, word: String) {
        for token in Token::lexer(&word) {
            self.analyze(token);
        }

        // Inform the user of the number of elements still in stack
        println!("{} elements in stack", self.stack.len());
    }

    // Find the index of the stack at which the function declaration ends
    #[inline]
    fn extract_function(
        &mut self,
        function_name: &String,
        arity: usize,
        mut index: usize,
    ) -> Found {
        let mut to_copy = 1;

        while to_copy > 0 && index > 0 {
            match &self.stack[index - 1] {
                Identifier(name) => {
                    // Check for self reference (for recursion)
                    if name.eq(function_name) {
                        to_copy += arity;
                        to_copy -= 1;
                    } else {
                        // Check table
                        match self.table.get(name) {
                            Some(Function(arity, _)) | Some(Iterative(arity, _, _, _)) => {
                                to_copy += arity;
                                to_copy -= 1;
                            }
                            _ => to_copy -= 1,
                        }
                    }
                }

                Number(_) | Argument(_) => to_copy -= 1,

                Plus | Minus | Times | Divide | PositiveMinus | IntegerDiv | Exp => to_copy += 1,

                If | ExpMod => to_copy += 2,

                _ => panic!("Corrupted stack"),
            }

            // Moves index
            index -= 1;
        }

        // If it managed to complete the expression with what was
        // found in stack, then index contains where to split
        if to_copy == 0 {
            FoundAt(index)
        } else {
            NotFound
        }
    }

    // Receive a token and decide what to do
    fn analyze(&mut self, token: Token) {
        match token {
            // Drop all errors
            Error => eprintln!("Dropped unrecognized token!"),

            // Compute and print top of the stack
            Return => {
                if let Some(mut num) = self.compute() {
                    num.normalize();
                    let (num, den) = num.into_parts();
                    if den.is_one() {
                        println!("> {}", num);
                    } else {
                        println!("> {}/{}", num, den);
                    }
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression");
                }
            }

            // 2645608968345021733469237830984 hello world for debugging
            // Computes the top of the stack and prints it as a string
            Format => {
                if let Some(mut num) = self.compute() {
                    num.normalize();
                    let (num, den) = num.into_parts();
                    // Turns the numerator into a vector of bytes and writes them to stdout
                    // In case of error it just prints a message
                    // The resulting string will be inverted, this makes it easier to build it
                    std::io::stdout()
                        .write(&(Stringer::from(num).collect::<Vec<u8>>())[..])
                        .unwrap_or_else(|_| {
                            eprintln!("Cannot print numerator string");
                            0
                        });
                    println!("");

                    // If the denominator is *not* one it does the same, on a new line
                    // Be carefull with non-coprimes, because they get normalized
                    if !den.is_one() {
                        std::io::stdout()
                            .write(&(Stringer::from(den).collect::<Vec<u8>>())[..])
                            .unwrap_or_else(|_| {
                                eprintln!("Cannot print numerator string");
                                0
                            });
                        println!("");
                    }
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression");
                }
            }

            // Computes the top of the stack and prints an approximation
            Approx => {
                if let Some(num) = self.compute() {
                    println!("> {:e}", num.to_f64());
                } else {
                    eprintln!("Incomplete expression");
                }
            }

            // Compute and print top of the stack
            // Put result back in stack
            Partial => {
                if let Some(mut num) = self.compute() {
                    println!("< {}", num);
                    num.normalize();
                    self.stack.push(Number(num));
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression");
                }
            }

            // Compute top of stack and duplicate it
            Duplicate => {
                if let Some(mut num) = self.compute() {
                    self.stack.push(Number(num.clone()));
                    num.normalize();
                    self.stack.push(Number(num));
                } else {
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute and print entire stack
            Flush => {
                for result in self.compute_all() {
                    if let Some(mut num) = result {
                        num.normalize();
                        let (num, den) = num.into_parts();
                        if den.is_one() {
                            println!("> {}", num);
                        } else {
                            println!("> {}/{}", num, den);
                        }
                    } else {
                        // Print error if arguments are missing
                        eprintln!("Incomplete expression");
                    }
                }
            }

            // Print all elements in stack without computing
            Print => {
                for token in &self.stack {
                    print!("{} ", token);
                }
                println!("");
            }

            // Flush all stack without computing it
            Empty => {
                self.stack.clear();
            }

            // Assign value to global variable
            // Drops previous value
            AssignVariable(mut name) => {
                if let Some(val) = self.compute() {
                    // Remove '=' from the name before inserting it
                    name.remove(0);
                    self.table.insert(name, Variable(val));
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            AssignFunction(name) => {
                let index = self.stack.len();

                // Split name from arity
                let mut parse = name.split('|');
                let function_name = String::from(parse.next().unwrap());
                let arity = parse.next().unwrap().parse().unwrap();

                if let FoundAt(index) = self.extract_function(&function_name, arity, index) {
                    // Insert a fake function for parsing recursive functions
                    self.table.insert(
                        function_name.clone(),
                        Object::Function(
                            arity,
                            ExecTree {
                                token: Number(Rational::zero()),
                                arguments: Vec::new(),
                            },
                        ),
                    );
                    // insert real function
                    self.table.insert(
                        function_name,
                        Function(arity, parse_tree(self.stack.split_off(index), &self.table)),
                    );
                } else {
                    eprintln!("Incomplete function declaration");
                }
            }

            AssignIterative(name) => {
                let mut index = self.stack.len();
                let mut indices = Vec::new();
                let mut found = true;

                // Split name from arity
                let mut parse = name.split('@');
                let function_name = String::from(parse.next().unwrap());
                let arity = parse.next().unwrap().parse().unwrap();

                let mut expressions = arity + 2;
                while expressions > 0 && found {
                    if let FoundAt(split_index) =
                        self.extract_function(&function_name, arity, index)
                    {
                        indices.push(split_index);
                        index = split_index;
                    } else {
                        found = false;
                        eprintln!("Incomplete function declaration");
                    }
                    expressions -= 1;
                }

                // Insert a fake function for parsing recursive functions
                // keep the previous object, in case
                let old = self.table.insert(
                    function_name.clone(),
                    Object::Function(
                        arity,
                        ExecTree {
                            token: Number(Rational::zero()),
                            arguments: Vec::new(),
                        },
                    ),
                );
                // If arity is correct
                if arity + 2 == indices.len() {
                    let mut expressions = Vec::new();

                    for index in indices {
                        expressions.push(self.stack.split_off(index));
                    }

                    let mut expressions: Vec<ExecTree> = expressions
                        .into_iter()
                        .map(|exp| parse_tree(exp, &self.table))
                        .rev()
                        .collect();
                    let condition = expressions.remove(arity + 1);
                    let last = expressions.remove(arity);
                    // Insert real function
                    self.table.insert(
                        function_name,
                        Iterative(arity, expressions, last, condition),
                    );
                } else {
                    // If arity is incorrect, put the old object back
                    if let Some(object) = old {
                        self.table.insert(function_name, object);
                    }
                }
            }

            // Eliminate top of stack without computing it
            Drop => {
                let mut to_drop = 1;
                while to_drop > 0 {
                    match self.stack.pop() {
                        None => to_drop = 0,

                        Some(Identifier(name)) => match self.table.get(&name) {
                            Some(Function(arity, _)) | Some(Iterative(arity, _, _, _)) => {
                                to_drop += arity;
                                to_drop -= 1;
                            }
                            _ => to_drop -= 1,
                        },

                        Some(Number(_)) | Some(Argument(_)) => to_drop -= 1,

                        Some(Plus) | Some(Minus) | Some(Times) | Some(Divide)
                        | Some(PositiveMinus) | Some(IntegerDiv) | Some(Exp) => to_drop += 1,

                        Some(If) | Some(ExpMod) => to_drop += 2,

                        _ => panic!("Corrupted stack"),
                    }
                }
            }

            // Push numbers and variables in stack
            _ => self.stack.push(token),
        }
    }

    // Compute top of stack and returns it
    // Returns None if the stack empties in advance
    #[inline]
    fn compute(&mut self) -> Option<Rational> {
        // Pop first expression
        let expression = clip_head(&mut self.stack, &self.table);

        // Return none if the expression was incomplete
        if expression.len() == 0 {
            return None;
        }

        // Parse execution tree from expression
        let tree = parse_tree(expression, &self.table);

        // Calculate value for exevution tree
        tree.reduce(&self.table, &Vec::new())
    }

    #[inline]
    fn compute_all(&mut self) -> Vec<Option<Rational>> {
        let mut all_trees = Vec::new();

        let mut found_incomplete = false;

        while self.stack.len() > 0 && !found_incomplete {
            let expression = clip_head(&mut self.stack, &self.table);

            if expression.len() > 0 {
                // Parse execution tree from expression
                let tree = parse_tree(expression, &self.table);

                // Calculate value for exevution tree
                all_trees.push(Some(tree));
            } else {
                found_incomplete = true;
                all_trees.push(None);
            }
        }

        all_trees
            .into_iter()
            .map(|tree| {
                if let Some(tree) = tree {
                    tree.reduce(&self.table, &Vec::new())
                } else {
                    None
                }
            })
            .collect()
    }
}
