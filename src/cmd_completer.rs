//! M-x command line completer — commands + table names.

use txv_core::complete::{Completer, Completion, CompletionVisitor};

/// All M-x built-in and Tcl commands.
const COMMANDS: &[&str] = &[
    "derive", "export", "freeze", "help", "into", "kiro", "plot", "quit", "run", "shell", "sql",
];

struct Entry {
    text: String,
    kind: &'static str,
}

impl Completion for Entry {
    fn text(&self) -> &str { &self.text }
    fn display(&self) -> &str { &self.text }
    fn kind(&self) -> &str { self.kind }
}

/// Completer for the M-x command line.
pub(crate) struct CmdCompleter;

impl Completer for CmdCompleter {
    fn complete(
        &self,
        input: &str,
        _cursor: usize,
        visitor: &mut CompletionVisitor<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trimmed = input.trim_start();
        for &cmd in COMMANDS.iter().filter(|c| c.starts_with(trimmed)) {
            let e = Entry { text: cmd.to_string(), kind: "command" };
            if !visitor(&e)? {
                return Ok(());
            }
        }
        Ok(())
    }
}
