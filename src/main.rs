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
use crate::calculator::Calculator;
use std::io::{stdin, BufRead};

fn main() {
    let reader = stdin();
    let mut calculator = Calculator::new();

    println!(
        "Welcome to rpn-c {}\n press Ctrl-D to quit...",
        env!("CARGO_PKG_VERSION")
    );

    let std_lib = String::from(include_str!("../std_lib.rpnl"));

    calculator.parse(std_lib);

    for line in reader.lock().lines() {
        calculator.parse(line.expect("IO Error occurred while reading from stdin"));
    }
}
