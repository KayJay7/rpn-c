# Reverse Polish Notation Calculator

A simple program that alows you to type in math expressions (even on multiple lines) in RPN and evaluate them.
The program keeps a table of golbal variables so you can store values for later use. All the numbers are stored as multiple precision rationals provided by the [GMP](https://gmplib.org/) library (through [rug](https://gitlab.com/tspiteri/rug)), so your calculations will be limited by just your memory (and rational numbers).

This little project started both because of necessity (I wanted a program for writing quick expressions from terminal, and I wanted it to compute big numbers), and to try out using a simple lexer and a simple stack machine. At first I just wanted it to compute simple arithmetics, but midway I started adding some quality of life feature like variables and other commands, there are still some features i plan to add.

## Syntax

* Expressions:
  * `(+|-)<some_decimal_number>(/<another_number>)` identifies a numeric constant (a fraction)
    * The sign is optional
    * The denominator is optional (you can't leave a pending `/` without denominator)
  * `<variable_name>` identifies a variable
  * `<exp1> <exp2> (+|-|*|/)` performs an arithmetic binary operation
    * Operations have fixed arity so parenthesis are not needed
  * `<exp1> <exp2> ~` perform a positive subtraction
    * If the result is lesser than `0`, it returns `0`
    * It returns the result otherwise
  * `<exp1> <exp2> <exp3> ?` if-then construct
    * If `<exp3>` equals `0`, drops `<exp2>` evaluates and returns `<exp1>`
    * If `<exp3>` *not* equals `0`, drops `<exp1>` evaluates and returns `<exp2>`
* Commands (commands will not be pushed in stack):
  * `<exp1> <function_name>|<arity>` declares a function of `<arity>` as `<exp1>`
  * `=<variable_name>` evaluates the expression on top of the stack and assigns its value to a variable
  * `=` evaluates the expression on top of the stack and prints it
  * `#` evaluates the expression on top of the stack and prints it, *and* pushes the result back in the stack
  * `:` prints the current stack
  * `>` evaluates and prints all the expressions on the stack (starting from top)
  * `<` evaluates and duplicate the expression on top of the stack
  * `!` drops the expression on top of the stack
    * Drops the entire expression, not just the last token
  * `%` drops the entire stack
  * `;` comments the rest of the line

## Completeness

With an extensive use of variables you should be able to evaluate most of the simple arithmetic expression with a little work on the user's end. More basic operations will be added in the near future, to lighten the user's work.
Powers, integer division, and remainders will be added for sure; while roots and logarithms will need more time (if they will be implemented) because they will cause a loss of precision (due to irrationality).
Infinite precision (with rational numbers) is a key element of the program, so anything that causes a fallback to floating point numbers (even Multiple-precision floating point numbers) will be neglected for the time being.

About scripting and Turing-Completeness. For now, the user's ability to script is limited to: concatenating some number and a script file, and execute the resulting stream.
It's still more than what your usual 4-op calculator can do, but it's not enough; the language is not Turing complete, it can neither iterate (which requires scoping and local variables) nor recurse and function composition (which require to have user defined functions in the first place). Around the time I decided to add variables, I also decided that TC is a necessity so stay tuned for that.

## Future developement

Near future:
* [x] Commenting
* [ ] Some more basic operations
  * [ ] Powers
  * [ ] Integer division
  * [ ] Remainder
* [ ] First crates.io release
* [ ] User defined functions
  * [x] `if-else`
  * [ ] Recursion
* [ ] Proof of Turing-Completeness
* [ ] A decent prompt (with history)
* [ ] Input from multiple files
* [ ] Output to file (silent mode)

Maybe one day:
* [ ] Approximation of *some* irrational operations
  * [ ] Approximation of irrational constants like pi, phi, e, log_2(10)
* [ ] Speeding up tail recursion (basically iteration)
* [ ] Upgrading to a real lalr (that kinda defeats the whole point)
* [ ] Programming an actual compiler
