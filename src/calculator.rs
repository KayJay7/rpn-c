use logos::Logos;
use rayon::prelude::*;
use rug::Rational;
use std::collections::HashMap;
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

    /*#[regex("\\\\")]
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
                let mut to_copy = 1;
                let mut i = self.stack.len();

                // Split name from arity
                let mut parse = name.split('|');
                let function_name = String::from(parse.next().unwrap());
                let arity = parse.next().unwrap().parse().unwrap();

                while to_copy > 0 && i > 0 {
                    match &self.stack[i - 1] {
                        Identifier(name) => {
                            // Check for self reference (for recursion)
                            if name.eq(&function_name) {
                                to_copy += arity - 1;
                            } else {
                                // Check table
                                match self.table.get(name) {
                                    Some(Function(arity, _)) => to_copy += arity - 1,
                                    _ => to_copy -= 1,
                                }
                            }
                        }

                        Number(_) | Argument(_) => to_copy -= 1,

                        Plus | Minus | Times | Divide | PositiveMinus => to_copy += 1,

                        If => to_copy += 2,

                        _ => panic!("Corrupted stack"),
                    }

                    // Moves index
                    i -= 1;
                }

                if to_copy == 0 {
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
}

struct ExecTree {
    token: Token,
    arguments: Vec<ExecTree>,
}

fn clip_head(stack: &mut Vec<Token>, table: &HashMap<String, Object>) -> Vec<Token> {
    let mut to_copy = 1;
    let mut i = stack.len();

    while to_copy > 0 && i > 0 {
        match &stack[i - 1] {
            Identifier(name) => {
                // Check table
                match table.get(name) {
                    Some(Function(arity, _)) => to_copy += arity - 1,
                    _ => to_copy -= 1,
                }
            }

            Number(_) | Argument(_) => to_copy -= 1,

            Plus | Minus | Times | Divide | PositiveMinus => to_copy += 1,

            If => to_copy += 2,

            _ => panic!("Corrupted stack"),
        }

        // Moves index
        i -= 1;
    }

    if to_copy == 0 {
        stack.split_off(i)
    } else {
        eprintln!("Incomplete expression, preserved stack");
        Vec::new()
    }
}

fn parse_tree(stack: Vec<Token>, table: &HashMap<String, Object>) -> ExecTree {
    let mut arguments = Vec::new();

    for token in stack {
        match token {
            Identifier(ref name) => match table.get(name) {
                Some(Function(arity, _)) => {
                    let len = arguments.len();
                    let args = arguments.split_off(len - arity);
                    arguments.push(ExecTree {
                        token,
                        arguments: args,
                    });
                }
                _ => {
                    arguments.push(ExecTree {
                        token,
                        arguments: Vec::new(),
                    });
                }
            },

            Number(_) | Argument(_) => {
                arguments.push(ExecTree {
                    token,
                    arguments: Vec::new(),
                });
            }

            Plus | Minus | Times | Divide | PositiveMinus => {
                let len = arguments.len();
                let args = arguments.split_off(len - 2);
                arguments.push(ExecTree {
                    token,
                    arguments: args,
                });
            }

            If => {
                let len = arguments.len();
                let args = arguments.split_off(len - 3);
                arguments.push(ExecTree {
                    token,
                    arguments: args,
                });
            }

            _ => panic!("Corrupted stack"),
        }
    }

    arguments.pop().unwrap()
}

impl ExecTree {
    // The result needs to be optional because
    // we don't know in advance if a function contains errors
    pub fn reduce(self, table: &HashMap<String, Object>) -> Option<Rational> {
        let ExecTree {
            token,
            mut arguments,
        } = self;

        match token {
            If => {
                // The if-else statement will not evaluate all of it's arguments
                let condition = arguments.pop().unwrap().reduce(table);

                if let Some(condition) = condition {
                    if condition.cmp0() == std::cmp::Ordering::Equal {
                        // Execute the right arm
                        arguments.pop().unwrap().reduce(table)
                    } else {
                        // Drop the right arm
                        arguments.pop();
                        // Execute the left arm
                        arguments.pop().unwrap().reduce(table)
                    }
                } else {
                    None
                }
            }

            Number(value) => Some(value),

            // Arithmetic operations
            _ => {
                // Start by executing every (2) argument
                let mut args: Vec<Option<Rational>> = arguments
                    .into_par_iter()
                    .map(|arg| arg.reduce(table))
                    .collect();

                let b = args.pop();
                let a = args.pop();

                if let (Some(Some(a)), Some(Some(b))) = (a, b) {
                    match token {
                        Plus => Some(a + b),
                        Minus => Some(a - b),
                        Times => Some(a * b),
                        Divide => Some(a / b),
                        PositiveMinus => {
                            let c = a - &b;
                            if c.cmp0() != std::cmp::Ordering::Less {
                                Some(c)
                            } else {
                                Some(Rational::from(0))
                            }
                        }

                        // All the other tokens will never enter the tree
                        _ => panic!("Corrupted stack"),
                    }
                } else {
                    None
                }
            }
        }
    }
}

impl Calculator {
    // Compute top of stack and returns it
    // Returns None if the stack empties in advance
    fn compute(&mut self) -> Option<Rational> {
        // Pop first expression
        let expression = clip_head(&mut self.stack, &self.table);

        // Parse execution tree from expression
        let tree = parse_tree(expression, &self.table);

        // Calculate value for exevution tree
        tree.reduce(&self.table)
    }
}
