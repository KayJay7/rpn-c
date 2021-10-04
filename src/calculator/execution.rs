use super::Token;
use super::Token::*;
use num_traits::{One, Zero};
use ramp::rational::Rational;
use std::collections::HashMap;
use Object::*;

#[derive(PartialEq, Clone)]
pub enum Object {
    Variable(Rational),
    Function(usize, ExecTree),
    Iterative(usize, Vec<ExecTree>, ExecTree, ExecTree),
}

#[derive(PartialEq, Clone)]
pub struct ExecTree {
    pub token: Token,
    pub arguments: Vec<ExecTree>,
}

#[inline]
pub fn parse_tree(stack: Vec<Token>, table: &HashMap<String, Object>) -> ExecTree {
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
        // If the recursive calls to reduce() used in the If, Function, and Iterative branches were
        // optimised as tail calls, all tail calls in rpn-l would also be optimised; the compiler
        // can't optimise those calls because Functions creates a new vector to borrow, which will
        // be destroyed after the call, so the actual last operation in the branch is the
        // deallocation, no the call. We can manually optimise these calls because when we are
        // dealing with a tail call, we only need to keep the arguments of the current function,
        // and if it's not a tail call, a previous stack frame is already holding the current
        // arguments safe, so we are not replacing anything when we reassign our arguments.
        // We are implementing a loop that when possible, just mutates the arguments and loops over
        // instead of calling a different function, similarly to how TCO is done in assembly; we
        // also need a variable where to keep our arguments when we optimise a tail call.
        // It gets a bit spaghetti.

        // Estract token and arguments from self (so you can move them indipendently)
        // they need to be mutable because they will be modified in loop (part of the TCO)
        let mut token = &self.token;
        // These arguments are the branches of the tree
        let mut arguments = &self.arguments;
        // These are the arguments of the current function
        let mut args = args;
        // This holds the arguments of a function that is being called
        // if it's a tail call, the arguments will override the previous call's ones
        let mut func_args: Vec<Option<Rational>>;

        // The loop will just go on unless a return gets called,
        // tail calls will just modify the argument variables and let the loop go on
        // other operations will just make some recursive calls and return the combined result
        loop {
            match token {
                If => {
                    // The if-else statement will not evaluate all of it's arguments
                    let condition = arguments[2].reduce(table, args);

                    if let Some(condition) = condition {
                        if condition.is_zero() {
                            // Execute the right arm
                            // This would be a tail call
                            token = &arguments[1].token;
                            arguments = &arguments[1].arguments;
                        } else {
                            // Execute the left arm
                            // This would be a tail call
                            token = &arguments[0].token;
                            arguments = &arguments[0].arguments;
                        }
                    } else {
                        // Stops in case of errors
                        return None;
                    }
                }

                Number(value) => {
                    return Some(value.clone());
                }

                Identifier(name) => {
                    if let Some(id) = table.get(name) {
                        match id {
                            Variable(value) => {
                                return Some(value.clone());
                            }
                            Function(arity, ops) => {
                                // Stop for invalid input before evaluating arguments
                                if arguments.len() != *arity {
                                    return None;
                                }

                                // Start by executing every argument
                                func_args = arguments
                                    .into_iter()
                                    .map(|arg| arg.reduce(table, args))
                                    .collect();

                                if func_args.iter().filter(|arg| arg.is_none()).count() > 0 {
                                    return None;
                                }

                                // This would be a tail call
                                token = &ops.token;
                                arguments = &ops.arguments;
                                args = &func_args;
                            }
                            Iterative(arity, exps, last, cond) => {
                                let mut stop = false;

                                // Stop for invalid input before evaluating arguments
                                if arguments.len() != *arity {
                                    return None;
                                }

                                // Start by executing every argument
                                func_args = arguments
                                    .into_iter()
                                    .map(|arg| arg.reduce(table, args))
                                    .collect();

                                // Iter untill cond returns a 0 (stop == true)
                                // Don't iter if cond returns None
                                while let (Some(value), false) =
                                    (run_function(cond, &func_args, table), stop)
                                {
                                    // Check for 0
                                    if !value.is_zero() {
                                        // Calculate new arguments from previous
                                        func_args = exps
                                            .iter()
                                            .map(|exp| run_function(&exp, &func_args, table))
                                            .collect();
                                    } else {
                                        // Set flag if 0
                                        stop = true;
                                    }
                                }
                                if func_args.iter().filter(|arg| arg.is_none()).count() > 0 {
                                    return None;
                                }

                                // This would be a tail call
                                token = &last.token;
                                arguments = &last.arguments;
                                args = &func_args;
                            }
                        }
                    } else {
                        return None;
                    }
                }

                Argument(index) => {
                    // Check index and return argument (if valid)
                    return if let Some(arg) = args.get(*index) {
                        arg.clone()
                    } else {
                        eprintln!("Invalid argument");
                        None
                    };
                }

                ExpMod => {
                    // Evaluates arguments
                    let a = arguments[0].reduce(table, args);
                    let b = arguments[1].reduce(table, args);
                    let c = arguments[2].reduce(table, args);

                    return if let (Some(a), Some(b), Some(c)) = (a, b, c) {
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
                    };
                }

                // Arithmetic operations, all binary operations
                _ => {
                    // Evaluates arguments
                    let a = arguments[0].reduce(table, args);
                    let b = arguments[1].reduce(table, args);

                    // Execute only if both arguments computed
                    // One 'Some' is for the pop operation (it will never be None)
                    return if let (Some(a), Some(b)) = (a, b) {
                        match token {
                            Plus => Some(a + b),
                            Minus => Some(a - b),
                            Times => Some(a * b),
                            Divide => {
                                if !b.is_zero() {
                                    Some(a / b)
                                } else {
                                    eprintln!("Cannot divide by zero");
                                    None
                                }
                            }
                            PositiveMinus => {
                                let c = a - &b;
                                if c > Rational::zero() {
                                    Some(c)
                                } else {
                                    Some(Rational::zero())
                                }
                            }
                            IntegerDiv => {
                                if !b.is_zero() {
                                    let (num, den) = (a / b).into_parts();
                                    Some(Rational::from(num / den))
                                } else {
                                    eprintln!("Cannot divide by zero");
                                    None
                                }
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
                    };
                }
            }
        }
    }
}

#[inline]
fn run_function(
    ops: &ExecTree,
    args: &Vec<Option<Rational>>,
    table: &HashMap<String, Object>,
) -> Option<Rational> {
    // Check is some arguments didn't compute
    if args.iter().filter(|arg| arg.is_none()).count() > 0 {
        return None;
    }
    // Execute tree
    ops.reduce(table, args)
}
