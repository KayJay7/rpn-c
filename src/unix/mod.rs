pub fn std_lib() -> String {
    String::from(include_str!("../../std_lib.rpnl"))
}

pub fn history_file() -> &'static str {
    "~/.rpnc_history"
}
