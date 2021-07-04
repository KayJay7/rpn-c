# Reverse Polish Notation Calculator

A simple program that alows you to type in math expressions (even on multiple lines) in RPN and evaluate them.
The program keeps a table of golbal variables so you can store values for later use. All the numbers are stored as multiple precision rationals provided by the [RAMP](https://github.com/Aatch/ramp) library, so your calculations will be limited by just your memory (and rational numbers).
`rpn-c` is also parallelized using [rayon](https://github.com/rayon-rs/rayon), so you can evaluate multiple expressions at the same time.

This little project started both because of necessity (I wanted a program for writing quick expressions from terminal, and I wanted it to compute big numbers), and to try out using a simple lexer and a simple stack machine (used only up to version 0.1.1). At first I just wanted it to compute simple arithmetics, but midway I started adding some quality of life feature like variables and other commands, there are still some features i plan to add.

Get it from [crates.io](https://crates.io/crates/rpn-c) with:
```sh
cargo install rpn-c
```

#### Hello, World!

```rpn-l
; "Hello, World!" -> "!dlroW ,olleH" reverse the string
; "!dlorW ,olleH" -> "21 64 6c 72 6f 57 20 2c 6f 6c 6c 65 48" convert each character into a byte
; "21 64 6c 72 6f 57 20 2c 6f 6c 6c 65 48" -> "2645608968345021733469237830984" convert it single integer
; use '&' to print
2645608968345021733469237830984 &
```

#### Notes on building

Building requires a nightly Rust toolchain, because `RAMP` uses nightly features in order to get better performances (namely: lazy_statics, intrinsics, inline assembly). Also `RAMP` doesn't support cross-compilation, but that's a minor inconvenience.
Also, this crate assumes that you are compiling for your local machine, and uses the flag `target-cpu=native` to get better performance by automatically enabling cpu-dependent features, like vectorization. This doesn't allow crosscompilation, if you want to crosscompile for a different architecture, you must select a different target cpu. Please notice that crosscompilation has not been tested.

## Syntax (rpn-l)

rpn-l is the language used by (and developed for) rpn-c. It's not really user friendly, but it works, and will allow you to write your own scripts and functions for your quick calculation needs.
Every expression (and most command) are defined with fixed arity so you don't need parenthesis. The only two exceptions (`>` and `@`), still have known arity.

rpn-l statements are composed from two types of tokens: expressions, which can be composed with other expressions to for new ones; and commands, which cannot be composed but sometime requires to be preceded by an expression.
All expression tokens get pushed on top of the stack from left to right, but are not evaluated. When (always from left to right) rpn-c encounters a command, this can cause the evaluation of the last expression pushed to the stack, or some other side effects.
Commands cause actions, they aren't pushed in stack, this means that they cannot get called from inside a function.

rpn-c maintains a table of all the identifiers and their meaning, only commands can alter this table, making it immutable for expression and functions.
This looks like a limitation, but immutability allows the evaluation tree to be executed in parallel.

* Expressions:
  * `(+|-)<some_decimal_number>(/<another_number>)` identifies a numeric constant (a fraction)
    * The sign is optional
    * The denominator is optional (you can't leave a pending `/` without denominator)
  * `<variable_name>` identifies a variable
  * `<exp0> <exp1> (+|-|*|/)` performs an arithmetic binary operation
    * Operations have fixed arity so parenthesis are not needed
  * `<exp0> <exp1> ~` perform a positive subtraction
    * If the result is lesser than `0`, it returns `0`
    * It returns the result otherwise
  * `<exp0> <exp1> \` perform an Euclidean (or integer) division
    * Performs a divizion and floors the result
    * Will always return an integer
  * `<exp0> <exp1> ^` perform an exponentiation
    * To remain in rational numbers, the floored absolute value of `<exp1>` is used as exponent
  * `<exp0> <exp1> <exp2> _` performs an exponentiation in modulo `<exp2>`
    * To remain in rational numbers, the floored absolute values of `<exp1>` and `<exp2>` are used
  * `<exp0> <exp1> <exp2> ?` if-then construct
    * If `<exp2>` *not* equals `0`, drops `<exp1>` evaluates and returns `<exp0>`
    * If `<exp2>` equals `0`, drops `<exp0>` evaluates and returns `<exp1>`
  * `$<some_number>` identifies an argument
    * Arguments can only be used inside of functions
  * `<exp0> <exp1> ... <function_name>` calls a function
    * Each `<expN>` corresponds to the argument `$N`
* Commands
  * `<exp0> <function_name>|<arity>` declares a function of `<arity>` as `<exp1>`
    * Functions are evaluated when they get executed, if an identifier change its meaning, the functions that refere to it will change behaviour, remember to update them
    * Sometimes you might want to define a function, refere it from another, than change the first function
      * This enables mutual recursion between functions
      * Remember to maintain the same arity or this will break the other function
  * `<exp0> <exp1> ... <expN-1> <expN> <expN+1> <function_name>@<arity>` declares an iterative function of `<arity>` `N`
    * Equivalent to `<exp0> <exp1> ... <expN-1> <function_name> <expN> <expN+1> ? <function_name>|<arity>`, but better
    * It's preferable to use iterative functions when possible, they are faster and don't cause stack overflows
      * Most functions can usually be rewritten this way, the ones that don't are often too complex to be calculated on big inputs in the first place
  * `<exp0> =<variable_name>` evaluates the expression on top of the stack and assigns its value to a variable
  * `<exp0> =` evaluates the expression on top of the stack and prints it
  * `<exp0> #` evaluates the expression on top of the stack and prints it, *and* pushes the result back in the stack
  * `:` prints the current stack
  * `>` evaluates and prints all the expressions on the stack (starting from top)
  * `<exp0> <` evaluates and duplicate the expression on top of the stack
  * `<exp0> &` evaluates `<exp0>` and prints it as a string
    * Reads the numerator per byte, from the least significant, and writes them to stdout
    * If the denominator is not 1, prints it on a new line
  * `<exp0> !` drops the expression on top of the stack
    * Drops the entire expression, not just the last token
  * `%` drops the entire stack
  * `;<some_comment>` comments the rest of the line

### std_lib

`rpn-c` includes a standard library that gets automatically loaded, this library contains several common math operation, mostly for natural numbers.

* Functions
  * `n floor` rounds `n` to the biggest integer lesser or equal than `n`
  * `n abs` calculates the absolute value of `n`
  * `n fib` calculates the `n`-th Fibonacci number
  * `n m mod` calculates the remainder of `n/m`
  * `n phi` approximates phi using Fibonacci numbers, the bigger `n` the more accurate the result
  * `n fact` calculates `n!`
  * `n k bin` calculates the binomial coefficient `n` over `k`
  * `n gsum` calculates the sum of the first `n` integers
  * `a b sift` calculates the sum of all the integer between a and b (included)
  * `n m ack` calculates the Ackermann function of `n` and `m`; most likely, it won't succed in an useful amount of time
  * `c s cons` puts the character `c` before the string `s`
  * `s1 s2 cat` concatenates string `s1` with string `s2`
  * `s reverse` reverses string `s`
* Variables
  * `lf` line feed
  * `cr` carriage return
  * `chara` character `'a'`
  * `charA` character `'A'`
  * `char0` character `'0'`

## Completeness

From version 0.1.1, rpn-l is Turing-Complete, so *theoretically* it can compute anything computable, but there'sstill work to do. The language still needs more features to ease the users work.
Powers, integer division, and remainders will be added for sure; while roots and logarithms will need more time (if they will be implemented) because they will cause a loss of precision (due to irrationality).
Infinite precision (with rational numbers) is a key element of the program, so anything that causes a fallback to floating point numbers (even Multiple-precision floating point numbers) will be neglected for the time being.

About scripting. For now, the user's ability to script is limited to: concatenating some number and a script file, and pipe that into rpn-c.
It's still more than what your usual 4-op calculator can do, but it's not enough. More features amied to scripting and working with library will be added in future.

## Proof of Turing-Completeness and Equivalence

The completeness of the language will be proved by simulating the primitives and the behaviour of the operators required for the construction μ-recursive functions, using a subset of the actual rpn-l language.
The subset consists of the operators `+` and `~`, the definition of N-ary functions, and the integer literals `0` and `1`; the `=` command is not needed for this proof, but it's needed to run the functions defined this way.
Other features of the language are not needed for completeness but make the language more usable.

### Primitive functions

#### Zero function

A function `zero` that receives N arguments and returns 0.

```rpn-l
0 zero|N
```

#### Successor function

A function `S` that increments its one argument by one.

```rpn-l
$0 1 + S|1
```

#### Projection function (identity function)

A function `P` that receives N arguments and returns the I-th argument

```rpn-l
$I P|N
```

### Operators

#### Composition operator

It's possible to define a K-ary function `g` by composing K N-ary functions `fk` and one K-ary function `h`.

```rpn-l
$0 ... $N-1 f1
...
$0 ... $N-1 fK
  h g|N
```

#### Primitive recursion operator

It is possible to define a K+1-ary primitive recursive function `f` given the K-ary function `g` for the base case and the K+2-ary function `h` for the recursive case.

```rpn-l
$0 1 ~ 
$0 1 ~ $1 ... $K f  ; Recursive call
$1 ... $K
  h                 ; Recursive case
  $1 ... $K g       ; Base case
  $0
    ? f|K+1
```

#### μ-operator

Given a K+1-ary function `f`, it is possible to write a K-ary function `mu-f` that receives K arguments and finds the smallest value (starting from 0), that (along with the other K arguments) causes `f` to return 0.

```rpn-l
$0 1 + $1 ... $K mu-f_rec       ; Recursive case
$0                              ; Found minimum
$0 ... $k f                     ; Test for zero
  ? mu-f_rec|K+1                ; Auxiliary function

0 $0 ... $K-1 mu-f_rec mu-f|K   ; μ-function
```

Alternatively:

```rpn-l
$0 1 + $1 ... $K                ; Calculate next arguments
$0                              ; Found minimum
$0 ... $K f                     ; Test for 0
  mu-f_aux@K+1                  ; Auxiliary function

0 $0 ... $k-1 mu-f_aux mu-f|K   ; μ-function
```

#### Conclusion

Results:
* μ-recursive functions are proved to be Turing Equivalent
  * Being able to simulate them makes rpn-l complete
* A computer is able to simulate rpn-l (rpn-c runs on a computer)
  * Every rpn-l function is also Turing-Computable
* From the above statements follows that rpn-l is Turing-Equivalent

## Why `RAMP` and not `GMP`?

Up until version 0.1.4, `rpn-c` was based on `GMP` using the `rug` crate as a safe interface over the C++ library. GMP *did* grant better performance on some situations, probably in general (due to its well-known maturity).
But RAMP provides comparable performances to GMP (at least in Linux x86_64, where it performs better), plus it is more ergonomic and (most important) does not require a GNU environment to build.

Switching back to GMP will be considered in the future, if performance becomes an issue. In that case, prebuild executables for Windows will be provided as releases on GitHub. Untill then, RAMP will be the library of choice for this project.

For your Rust project, if you don't mind needing a GNU environment to build, and the rug's ergonomics is not an issue, that is probably the best choice, due to it's indiscuted better preformance.

## Future developement

Near future:
* [x] Commenting
* [ ] Some more basic operations (paused)
  * [x] Powers
  * [x] Integer division (required for 0.2.0)
  * [x] Remainder
* [x] User defined functions
  * [x] `if-else`
  * [x] Recursion
* [x] Proof of Turing-Completeness
* [x] First crates.io release
* [ ] Solve the stack overflow issue
  * Rewrite the executor so that it doesn't use recursion
    * Would probably cause less parallelization
  * [x] Allow writing iterative functions (required for 0.2.0)
    * Enables writing functions that don't overflow with few thousands of iterations
* [x] Add parallelization
* [x] Switch to a non GMP-dependent crate
* [ ] A decent prompt (with history)
* [ ] Input from multiple files
  * [ ] Support for shebang
* [ ] Output to file (silent mode)

Maybe one day:
* [ ] Approximation of *some* irrational operations
  * [ ] Approximation of irrational constants like pi, phi, e, log_2(10)
* [ ] Speeding up tail recursion
  * [x] Add some form of iteration without recursion
* [ ] Upgrading to a real LALR (that kinda defeats the whole point)
* [ ] Programming an actual compiler

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
