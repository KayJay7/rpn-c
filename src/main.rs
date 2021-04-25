// Copyright ⓒ 2021 Alvise Bruniera
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod calculator;
mod input;
use crate::calculator::Calculator;
use crate::input::new_editor;
use rustyline::error::ReadlineError;

fn main() {
    let mut calculator = Calculator::new();
    let mut rl = new_editor();

    println!(
        "Welcome to rpn-c {}\n press Ctrl-D to quit...",
        env!("CARGO_PKG_VERSION")
    );

    let std_lib = String::from(include_str!("../std_lib.rpnl"));

    calculator.parse(std_lib);

    /*loop {
        calculator.parse(
            rl.readline("λ> ")
                .expect("IO Error occurred while reading from stdin"),
        );
    }*/
    loop {
        let readline = rl.readline("λ> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                calculator.parse(line);
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}
