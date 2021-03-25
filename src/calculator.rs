use logos::Logos;
//use rug::ops::Pow;
use rug::Rational;
use std::collections::HashMap;
//use std::convert::TryFrom;
use std::fmt;
use std::string::String;

// Readable tokens from command line
#[derive(Logos, Debug, PartialEq, Clone)]
enum Token {
    #[regex("[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    Identifier(String),

    #[regex("=[a-zA-Z]([a-zA-Z0-9]|-[a-zA-Z0-9]|_[a-zA-Z0-9])*", |lex| String::from(lex.slice()))]
    AssignVariable(String),

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
    Function(u64, Vec<Token>),
}

// Implement Display for printing
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Number(num) => write!(f, "{}", num),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Times => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::If => write!(f, "?"),
            Token::Identifier(name) => write!(f, "{}", name),
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
            Token::Error => eprintln!("Dropped unrecognized token!"),

            // Compute and print top of the stack
            Token::Return => {
                if let Some(num) = self.compute() {
                    println!("> {}", num);
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute and print top of the stack
            // Put result back in stack
            Token::Partial => {
                if let Some(num) = self.compute() {
                    println!("< {}", num);
                    self.stack.push(Token::Number(num));
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute top of stack and duplicate it
            Token::Duplicate => {
                if let Some(num) = self.compute() {
                    self.stack.push(Token::Number(num.clone()));
                    self.stack.push(Token::Number(num));
                } else {
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Compute and print entire stack
            Token::Flush => {
                while self.stack.len() > 0 {
                    self.analyze(Token::Return);
                }
            }

            // Print all elements in stack without computing
            Token::Print => {
                for token in &self.stack {
                    print!("{} ", token);
                }
                println!("");
            }

            // Flush all stack without computing it
            Token::Empty => {
                self.stack.clear();
            }

            // Assign value to global variable
            // Drops previous value
            Token::AssignVariable(mut name) => {
                if let Some(val) = self.compute() {
                    // Remove '=' from the name before inserting it
                    name.remove(0);
                    self.table.insert(name, Object::Variable(val));
                } else {
                    // Print error if arguments are missing
                    eprintln!("Incomplete expression, dropped stack");
                }
            }

            // Eliminate top of stack without computing it
            Token::Drop => {
                let mut to_drop = 1;
                while to_drop > 0 {
                    match self.stack.pop() {
                        None => to_drop = 0,

                        Some(Token::Identifier(name)) => match self.table.get(&name) {
                            Some(Object::Function(arity, _)) => to_drop -= arity - 1,
                            _ => to_drop -= 1,
                        },

                        Some(Token::Number(_)) => to_drop -= 1,

                        Some(Token::Plus) | Some(Token::Minus) | Some(Token::Times)
                        | Some(Token::Divide) => to_drop += 1,

                        Some(Token::If) => to_drop += 2,

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
                Token::Number(num) => Some(num),

                // Return variable's value
                Token::Identifier(name) => match self.table.get(&name) {
                    Some(Object::Variable(value)) => Some(value.clone()),
                    Some(Object::Function(arity, ops)) => None,
                    None => {
                        eprintln!("Undefined name: {}", name);
                        None
                    }
                },

                Token::If => {
                    let test = self.compute();

                    if let Some(test) = test {
                        if test.cmp0() == std::cmp::Ordering::Equal {
                            self.analyze(Token::Drop);
                            self.compute()
                        } else {
                            let res = self.compute();
                            self.analyze(Token::Drop);
                            res
                        }
                    } else {
                        None
                    }
                }

                // This tokens will never be on stack
                Token::Flush
                | Token::Drop
                | Token::Empty
                | Token::AssignVariable(_)
                | Token::Duplicate
                | Token::Print
                | Token::Error
                | Token::Partial
                | Token::Return => panic!("Corrupted stack"),

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
                            Token::Plus => Some(a + &b),
                            Token::Minus => Some(a - &b),
                            Token::Times => Some(a * &b),
                            Token::Divide => Some(a / &b),
                            /*Token::Power => {
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
