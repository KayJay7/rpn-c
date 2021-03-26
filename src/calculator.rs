use logos::Logos;
//use rug::ops::Pow;
use rug::Rational;
use std::collections::HashMap;
//use std::convert::TryFrom;
use std::fmt;
use std::string::String;
use Object::*;
use Token::*;

// Readable tokens from command line
#[derive(Logos, Debug, PartialEq, Clone)]
enum Token {
    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    Identifier(String),

    #[regex("=[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    AssignVariable(String),

    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*\\|[0-9]+", |lex| String::from(lex.slice()))]
    AssignFunction(String),

    #[regex("\\$[0-9]+", |lex| {
        let mut parse = lex.slice().split('$');
        parse.next();
        parse.next().unwrap().parse()
    })]
    Argument(usize),

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

    /*#[regex("\\^")]
    Power,

    #[regex("@")]
    PowerMod,

    #[regex("\\\\")]
    IntegerDiv,
    */
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

    #[error]
    #[regex(";.*", logos::skip)]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(PartialEq, Clone)]
enum Object {
    Variable(Rational),
    Function(usize, Vec<Token>),
}

// Implement Display for printing
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Number(num) => write!(f, "{}", num),
            Plus => write!(f, "+"),
            Minus => write!(f, "-"),
            Times => write!(f, "*"),
            Divide => write!(f, "/"),
            If => write!(f, "?"),
            PositiveMinus => write!(f, "~"),
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
    pub fn new() -> Calculator {
        Calculator {
            stack: Vec::new(),
            table: HashMap::new(),
        }
    }

    // To be called from main,
    // Parse a line into tokens and compute them
    pub fn parse(&mut self, word: String) {
        for token in Token::lexer(&word) {
            self.analyze(token);
        }

        // Inform the user of the number of elements still in stack
        println!("{} elements in stack", self.stack.len());
    }

    // Receive a token and decide what to do
    fn analyze(&mut self, token: Token) {
        match token {
            // Drop all errors
            Error => eprintln!("Dropped unrecognized token!"),

            // Compute and print top of the stack
            Return => {
                if let Some(num) = self.compute() {
                    println!("> {}", num);
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute and print top of the stack
            // Put result back in stack
            Partial => {
                if let Some(num) = self.compute() {
                    println!("< {}", num);
                    self.stack.push(Number(num));
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute top of stack and duplicate it
            Duplicate => {
                if let Some(num) = self.compute() {
                    self.stack.push(Number(num.clone()));
                    self.stack.push(Number(num));
                } else {
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute and print entire stack
            Flush => {
                while self.stack.len() > 0 {
                    self.analyze(Return);
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
                let mut to_drop = 1;
                let mut i = self.stack.len();

                // Split name from arity
                let mut parse = name.split('|');
                let function_name = String::from(parse.next().unwrap());
                let arity = parse.next().unwrap().parse().unwrap();

                while to_drop > 0 && i > 0 {
                    match &self.stack[i - 1] {
                        Identifier(name) => {
                            // Check for self reference (for recursion)
                            if name.eq(&function_name) {
                                to_drop += arity - 1;
                            } else {
                                // Check table
                                match self.table.get(name) {
                                    Some(Function(arity, _)) => to_drop += arity - 1,
                                    _ => to_drop -= 1,
                                }
                            }
                        }

                        Number(_) | Argument(_) => to_drop -= 1,

                        Plus | Minus | Times | Divide | PositiveMinus => to_drop += 1,

                        If => to_drop += 2,

                        _ => panic!("Corrupted stack"),
                    }

                    // Moves index
                    i -= 1;
                }

                if to_drop == 0 {
                    self.table
                        .insert(function_name, Function(arity, self.stack.split_off(i)));
                } else {
                    eprintln!("Incomplete function declaration, preserved stack");
                }
            }

            // Eliminate top of stack without computing it
            Drop => {
                let mut to_drop = 1;
                while to_drop > 0 {
                    match self.stack.pop() {
                        None => to_drop = 0,

                        Some(Identifier(name)) => match self.table.get(&name) {
                            Some(Function(arity, _)) => to_drop += arity - 1,
                            _ => to_drop -= 1,
                        },

                        Some(Number(_)) | Some(Argument(_)) => to_drop -= 1,

                        Some(Plus) | Some(Minus) | Some(Times) | Some(Divide)
                        | Some(PositiveMinus) => to_drop += 1,

                        Some(If) => to_drop += 2,

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
    fn compute(&mut self) -> Option<Rational> {
        if let Some(token) = self.stack.pop() {
            match token {
                // Return numers as is
                Number(num) => Some(num),

                // Return variable's value
                Identifier(name) => match self.table.get(&name) {
                    Some(Variable(value)) => Some(value.clone()),
                    Some(Function(arity, ops)) => None,
                    None => {
                        eprintln!("Undefined name: {}", name);
                        None
                    }
                },

                If => {
                    let test = self.compute();

                    if let Some(test) = test {
                        if test.cmp0() == std::cmp::Ordering::Equal {
                            self.analyze(Drop);
                            self.compute()
                        } else {
                            let res = self.compute();
                            self.analyze(Drop);
                            res
                        }
                    } else {
                        None
                    }
                }

                // This tokens will never be on stack
                Flush | Drop | Empty | AssignVariable(_) | AssignFunction(_) | Duplicate
                | Print | Error | Partial | Return => panic!("Corrupted stack"),

                Argument(_) => {
                    eprintln!("Arguments cannot be used outside of functions");
                    None
                }

                // Binary operators are the only tokens left
                _ => {
                    // Compute arguments
                    let b = self.compute();
                    let a = self.compute();

                    // If both computed sucesfully combine and return
                    if let (Some(a), Some(b)) = (a, b) {
                        match token {
                            // Operations on borrowed values may return incomplete results
                            // Using incomplete results might make long operations faster
                            Plus => Some(a + &b),
                            Minus => Some(a - &b),
                            Times => Some(a * &b),
                            Divide => Some(a / &b),
                            PositiveMinus => {
                                let c = a / &b;
                                if c.cmp0() > std::cmp::Ordering::Greater {
                                    Some(c)
                                } else {
                                    Some(Rational::from(0))
                                }
                            }
                            /*Power => {
                                if let Some(1) = b.denom().to_u32() {
                                    let numer = a.numer().pow(b.numer());
                                    let denom = a.denom().pow(b.numer());
                                    Some(Rational::from((numer, denom)))
                                } else {
                                    None
                                }
                            }*/
                            // At this point, the token can only be a binary operators
                            _ => panic!("Corrupted stack"),
                        }
                    } else {
                        // Return None if arguments didn't compute
                        None
                    }
                }
            }
        } else {
            // Return None if stack is empty
            None
        }
    }
}
