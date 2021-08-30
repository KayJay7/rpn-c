use std::borrow::Cow::{self, Borrowed, Owned};

use directories::ProjectDirs;
use lazy_static::lazy_static;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, Validator};
use rustyline::{Cmd, CompletionType, Config, Context, EditMode, Editor, KeyEvent};
use rustyline_derive::Helper;
use std::path::{Path, PathBuf};

lazy_static! {
    pub static ref DIRS: Option<ProjectDirs> = ProjectDirs::from("com", "rpn-lang", "rpn-c");
    pub static ref DATA_LOCAL_DIR: Option<PathBuf> = DIRS
        .as_ref()
        .and_then(|dirs| Some(Path::new(dirs.data_local_dir()).to_path_buf()));
    pub static ref HISTORY_PATH: Option<PathBuf> = DATA_LOCAL_DIR
        .as_ref()
        .and_then(|dir| Some(dir.join("history.txt")));
}

#[derive(Helper)]
pub struct MyHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MyHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[2m".to_owned() + hint + "\x1b[0m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

// Validation is not needed (not here anyway)
impl Validator for MyHelper {
    fn validate(
        &self,
        _ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        Ok(validate::ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

pub fn new_editor() -> Editor<MyHelper> {
    env_logger::init();

    // Build configuration
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();

    // Build helper struct
    let h = MyHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
    };

    // Configure editor with the above two
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
    rl.bind_sequence(KeyEvent::ctrl('d'), Cmd::EndOfFile);

    // If possible load history
    if let Some(path) = &*HISTORY_PATH {
        if !path.exists() {
            eprintln!("{}", path.to_str().unwrap());
        }
        if rl.load_history(&path).is_err() {}
    }

    rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{}\x1b[0m", "λ> ");

    rl
}
