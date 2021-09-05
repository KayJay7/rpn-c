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
use calculator::Calculator;
use input::{new_editor, Edit, DATA_LOCAL_DIR, HISTORY_PATH};
use rustyline::error::ReadlineError;
use std::fs::create_dir_all;

fn main() {
    // Makes sure data_local_dir exists
    if let Some(path) = &*DATA_LOCAL_DIR {
        // It's not important if there's no history
        create_dir_all(path).unwrap_or_else(|_| {});
    }

    // Creates calculator object and prompt
    let mut calculator = Calculator::new();
    let mut rl = new_editor();

    if let Some(path) = &*HISTORY_PATH {
        if !path.exists() {}
        rl.load_history(path)
            .unwrap_or_else(|_| eprintln!("Unable to create local data dir"));
    }

    // Print welcome
    println!(
        "Welcome to rpn-c {}\n press Ctrl-D to quit...",
        env!("CARGO_PKG_VERSION")
    );

    #[cfg(unix)]
    calculator.parse(String::from(include_str!("../std_lib.rpnl")));

    #[cfg(windows)]
    calculator.parse(String::from(include_str!("..\\std_lib.rpnl")));

    // REPL loop
    repl(calculator, &mut rl);

    // Save history in the same file, if possible
    if let Some(path) = &*HISTORY_PATH {
        rl.append_history(path)
            .unwrap_or_else(|_| eprintln!("Unable to append history"));
    }
}

#[inline]
fn repl(mut calculator: Calculator, rl: &mut Edit) {
    // REPL loop
    loop {
        let readline = rl.readline("λ> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                calculator.parse(line);
            }
            // Exit if the program is interrupted (Ctrl+C)
            Err(ReadlineError::Interrupted) => {
                break;
            }
            // Exit at end of file (which is caused by the end of a pipe or the input of Ctrl+D)
            Err(ReadlineError::Eof) => {
                break;
            }
            // Report any other error
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}
