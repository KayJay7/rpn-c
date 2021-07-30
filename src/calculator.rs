use logos::Logos;
use num_traits::{One, Zero};
use ramp::int::Int;
use ramp::rational::Rational;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::string::String;
use Found::*;
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
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

// Byte iterator for string printing
// with "buffering" to allow better unrolling
// Might cause an excess of up to 7 trailing zeroes
struct Stringer {
    num: Int,
    partial: u64,
    iter: usize,
}

impl Stringer {
    // Constructor, consumes the provided Int
    fn from(num: Int) -> Stringer {
        Stringer {
            num: num.abs(),
            partial: 0,
            // We could do withoud iter, and reduce the number of (possibly useless) 0-writes
            // but having it makes the loop more predictable
            // plus, those 0-writes might not be actually useless
            iter: 8,
        }
    }
}

impl Iterator for Stringer {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        // If all 8 bufferized bytes have been printed
        // extract new ones
        if self.iter == 8 {
            // Returns None if there are no more bytes to extract
            if self.num.is_zero() {
                return None;
            }

            // Otherwise extract 8 more using integer divisions and modulo
            let (q, r) = self.num.divmod(&Int::from(0x1_00_00_00_00_00_00_00_00u128));
            // Populate buffer with extracted bytes
            self.num = q;
            self.partial = u64::from(&r);
            self.iter = 0;
        }

        // Extract one byte from buffer, increase counter, and returns
        let shift = self.iter * 8;
        let byte = ((self.partial >> shift) & 255) as u8;
        self.iter += 1;
        Some(byte)
    }
}

fn from_string(string: &str) -> Rational {
    // Trims quotes and turns into iter
    let iter = string.as_bytes()[1..(string.len() - 1)].into_iter();
    let mut num = Int::zero();
    let mut esc = false;
    let mut esc2 = false;
    let mut prev = 0;
    let mut stack = Vec::new();

    // Parse escape sequences
    for ch in iter {
        // If not escaped, just stack it
        if !esc {
            // 92='\\'
            if *ch != 92 {
                stack.push(*ch);
            } else {
                esc = true;
            }
        } else {
            // Otherwise look at next character
            esc = false;
            if !esc2 {
                // If its a shortcut escape, stack it
                // Note that num was already shifted from the loop before
                match *ch {
                    110 => stack.push(10), // \n
                    114 => stack.push(13), // \r
                    116 => stack.push(09), // \t
                    92 => stack.push(92),  // \\
                    34 => stack.push(34),  // \"
                    // If its an hex escape, save it and read the next one to
                    _ => {
                        esc = true;
                        esc2 = true;
                        prev = from_hex(*ch) << 4;
                    }
                }
            } else {
                // Now that you have the second hex, stack the whole
                // Note that num was already shifted from the loop before
                esc = false;
                esc2 = false;
                stack.push(prev + from_hex(*ch));
            }
        }
    }

    // Puts characters into num
    // accelerated in a similar fashion to stringer
    while !stack.is_empty() {
        let mut partial: u64 = 0;
        let mut count = 0;
        let mut stop = false;

        // Stack at most 8 bytes into partial, and counts them
        // Using a while let would drop one byte every 8
        while count != 8 && !stop {
            if let Some(ch) = stack.pop() {
                partial <<= 8;
                partial += ch as u64;
                count += 1;
            } else {
                stop = true;
            }
        }

        // Make space for partial, and add it
        num <<= 8 * count;
        num += partial;
    }

    Rational::from(num)
}

fn from_hex(hex: u8) -> u8 {
    if hex >= 48 && hex <= 57 {
        hex - 48
    } else if hex >= 65 && hex <= 90 {
        hex - 55
    } else if hex >= 97 && hex <= 122 {
        hex - 87
    } else {
        0
    }
}

#[derive(PartialEq, Clone)]
enum Object {
    Variable(Rational),
    Function(usize, ExecTree),
    Iterative(usize, Vec<ExecTree>, ExecTree, ExecTree),
}

// Implement Display for printing
impl fmt::Display for Token {
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

enum Found {
    NotFound,
    FoundAt(usize),
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
                    println!("> {}", num.to_f64());
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
                        .into_par_iter()
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
}

#[derive(PartialEq, Clone)]
struct ExecTree {
    token: Token,
    arguments: Vec<ExecTree>,
}

fn clip_head(stack: &mut Vec<Token>, table: &HashMap<String, Object>) -> Vec<Token> {
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

fn parse_tree(stack: Vec<Token>, table: &HashMap<String, Object>) -> ExecTree {
    let mut arguments = Vec::new();

    // Builds the tree from the stack
    // Each token gets built into a tree node and put on an arguments stack
    // when building a node, it pops arguments from the stack an pass them to the node
    for token in stack {
        match token {
            Identifier(ref name) => match table.get(name) {
                Some(Function(arity, _)) | Some(Iterative(arity, _, _, _)) => {
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

            Plus | Minus | Times | Divide | PositiveMinus | IntegerDiv | Exp => {
                let len = arguments.len();
                let args = arguments.split_off(len - 2);
                arguments.push(ExecTree {
                    token,
                    arguments: args,
                });
            }

            If | ExpMod => {
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

    // At the end, the only argument in stack will be the root node of the computation
    arguments.pop().unwrap()
}

fn run_function(
    ops: &ExecTree,
    args: &Vec<Option<Rational>>,
    table: &HashMap<String, Object>,
) -> Option<Rational> {
    // Check is some arguments didn't compute
    if args.par_iter().filter(|arg| arg.is_none()).count() > 0 {
        return None;
    }
    // Execute tree
    ops.reduce(table, args)
}

// Tail recursive Fibonacci for testing
// $1 $0 $1 + $2 1 ~ fib_rec $1 $2 ? fib_rec|3 1 0 $0 fib_rec tfib|1
//
// Naive Fibonacci for testing
// $0 1 ~ nfib $0 2 ~ nfib + $0 $0 1 ~ ? nfib|1
//
// Iterative Fibonacci for testing
// $1 $0 $1 + $2 1 ~ $1 $2 fib_aux@3 1 0 $0 fib_aux fib|1
impl ExecTree {
    // The result needs to be optional because
    // we don't know in advance if a function contains errors
    pub fn reduce(
        &self,
        table: &HashMap<String, Object>,
        args: &Vec<Option<Rational>>,
    ) -> Option<Rational> {
        // Estract token and arguments from self (so you can move them indipendently)
        let token = &self.token;
        let arguments = &self.arguments;

        match token {
            If => {
                // The if-else statement will not evaluate all of it's arguments
                let condition = arguments[2].reduce(table, args);

                if let Some(condition) = condition {
                    if condition.is_zero() {
                        // Execute the right arm
                        arguments[1].reduce(table, args)
                    } else {
                        // Execute the left arm
                        arguments[0].reduce(table, args)
                    }
                } else {
                    None
                }
            }

            Number(value) => Some(value.clone()),

            Identifier(name) => {
                if let Some(id) = table.get(name) {
                    match id {
                        Variable(value) => Some(value.clone()),
                        Function(arity, ops) => {
                            // Stop for invalid input before evaluating arguments
                            if arguments.len() != *arity {
                                return None;
                            }

                            // Start by executing every argument
                            let args: Vec<Option<Rational>> = arguments
                                .into_par_iter()
                                .map(|arg| arg.reduce(table, args))
                                .collect();

                            // Run function with those arguments
                            run_function(ops, &args, table)
                        }
                        Iterative(arity, exps, last, cond) => {
                            let mut stop = false;

                            // Stop for invalid input before evaluating arguments
                            if arguments.len() != *arity {
                                return None;
                            }

                            // Start by executing every argument
                            let mut args: Vec<Option<Rational>> = arguments
                                .into_par_iter()
                                .map(|arg| arg.reduce(table, args))
                                .collect();

                            // Iter untill cond returns a 0 (stop == true)
                            // Don't iter if cond returns None
                            while let (Some(value), false) =
                                (run_function(cond, &args, table), stop)
                            {
                                // Check for 0
                                if !value.is_zero() {
                                    // Calculate new arguments from previous
                                    args = exps
                                        .par_iter()
                                        .map(|exp| run_function(&exp, &args, table))
                                        .collect();
                                } else {
                                    // Set flag if 0
                                    stop = true;
                                }
                            }

                            // Run the exit function on the last set of arguments
                            run_function(&last, &args, table)
                        }
                    }
                } else {
                    None
                }
            }

            Argument(index) => {
                // Check index and return argument (if valid)
                if let Some(arg) = args.get(*index) {
                    arg.clone()
                } else {
                    eprintln!("Invalid argument");
                    None
                }
            }

            ExpMod => {
                let mut args: Vec<Option<Rational>> = arguments
                    .into_par_iter()
                    .map(|arg| arg.reduce(table, args))
                    .collect();

                // Move args out of array (you can't add borrows)
                let c = args.pop();
                let b = args.pop();
                let a = args.pop();
                if let (Some(Some(a)), Some(Some(b)), Some(Some(c))) = (a, b, c) {
                    // Flooring and converting to Int
                    let (num, den) = a.into_parts();
                    let a = num / den;
                    let (num, den) = b.into_parts();
                    let b = (num / den).abs();
                    let (num, den) = c.into_parts();
                    let c = (num / den).abs();

                    Some(Rational::from(a.pow_mod(&b, &c)))
                } else {
                    None
                }
            }

            // Arithmetic operations
            _ => {
                // Start by executing every (2) argument
                let mut operands: Vec<Option<Rational>> = arguments
                    .into_par_iter()
                    .map(|arg| arg.reduce(table, args))
                    .collect();

                // Move args out of array (you can't add borrows)
                let b = operands.pop();
                let a = operands.pop();

                // Execute only if both arguments computed
                // One 'Some' is for the pop operation (it will never be None)
                if let (Some(Some(a)), Some(Some(b))) = (a, b) {
                    match token {
                        Plus => Some(a + b),
                        Minus => Some(a - b),
                        Times => Some(a * b),
                        Divide => Some(a / b),
                        PositiveMinus => {
                            let c = a - &b;
                            if c > Rational::zero() {
                                Some(c)
                            } else {
                                Some(Rational::zero())
                            }
                        }
                        IntegerDiv => {
                            let (num, den) = (a / b).into_parts();
                            Some(Rational::from(num / den))
                        }
                        Exp => {
                            //Flooring and converting to Int
                            let mut a = a;
                            let (num, den) = b.into_parts();
                            let mut b = (num / den).abs();
                            let mut result = Rational::one();
                            while !b.is_zero() {
                                if !b.is_even() {
                                    result *= &a;
                                }
                                b /= 2;
                                // Unfortunately we have to clone
                                // the size of a would double anyway
                                a *= a.clone();
                            }
                            Some(result)
                        }

                        // All the other tokens will never enter the tree
                        _ => panic!("Corrupted stack"),
                    }
                } else {
                    // Return None if an argument didn't compute
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

        // Return none if the expression was incomplete
        if expression.len() == 0 {
            return None;
        }

        // Parse execution tree from expression
        let tree = parse_tree(expression, &self.table);

        // Calculate value for exevution tree
        tree.reduce(&self.table, &Vec::new())
    }

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
            .into_par_iter()
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
