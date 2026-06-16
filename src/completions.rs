//! Completion engine — provides context-aware suggestions for the REPL.

use crate::engine::Engine;
use crate::registry::Registry;

/// A completion suggestion.
pub(crate) struct Completion {
    pub(crate) text: String,
    pub(crate) kind: CompletionKind,
}

#[derive(Clone, Copy)]
pub(crate) enum CompletionKind {
    Command,
    Table,
    Column,
}

const COMMANDS: &[&str] = &[
    "sql", "into", "plot", "derive", "export", "freeze", "run", "budget", "stats", "hist", "cdf", "corr",
];

/// Generate completions for the given input text.
pub(crate) fn complete(input: &str, engine: &Engine, registry: &Registry) -> Vec<Completion> {
    let trimmed = input.trim_start();

    // Empty or first word — complete commands.
    if !trimmed.contains(' ') {
        return complete_commands(trimmed);
    }

    // After first word — context-dependent.
    let first_space = trimmed.find(' ').unwrap_or(0);
    let cmd = &trimmed[..first_space];
    let rest = trimmed[first_space..].trim_start();

    match cmd {
        "sql" | "derive" => complete_tables_and_columns(rest, engine),
        "plot" => complete_plot_args(rest, engine, registry),
        "into" => complete_into_args(rest),
        "export" => complete_tables(engine),
        _ => complete_tables(engine),
    }
}

fn complete_commands(prefix: &str) -> Vec<Completion> {
    COMMANDS
        .iter()
        .filter(|c| c.starts_with(prefix))
        .map(|c| Completion {
            text: c.to_string(),
            kind: CompletionKind::Command,
        })
        .collect()
}

fn complete_tables(engine: &Engine) -> Vec<Completion> {
    let Ok(result) = engine.query("SHOW TABLES") else {
        return vec![];
    };
    result
        .rows
        .iter()
        .filter_map(|r| r.first())
        .map(|name| Completion {
            text: name.clone(),
            kind: CompletionKind::Table,
        })
        .collect()
}

fn complete_tables_and_columns(context: &str, engine: &Engine) -> Vec<Completion> {
    // If we see "FROM <table>" pattern, complete columns of that table.
    let upper = context.to_uppercase();
    if let Some(from_pos) = upper.rfind("FROM ") {
        let after = &context[from_pos + 5..];
        let table = after.split_whitespace().next().unwrap_or("").trim_matches('"');
        if !table.is_empty() {
            return complete_columns(table, engine);
        }
    }
    complete_tables(engine)
}

fn complete_columns(table: &str, engine: &Engine) -> Vec<Completion> {
    let sql = format!("SELECT column_name FROM information_schema.columns WHERE table_name = '{table}'");
    let Ok(result) = engine.query(&sql) else { return vec![] };
    result
        .rows
        .iter()
        .filter_map(|r| r.first())
        .map(|name| Completion {
            text: name.clone(),
            kind: CompletionKind::Column,
        })
        .collect()
}

fn complete_plot_args(rest: &str, engine: &Engine, registry: &Registry) -> Vec<Completion> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    match parts.len() {
        0 => {
            // Complete plot types.
            vec![
                Completion {
                    text: "bar".into(),
                    kind: CompletionKind::Command,
                },
                Completion {
                    text: "line".into(),
                    kind: CompletionKind::Command,
                },
            ]
        }
        1 => {
            // Complete table/node names.
            let mut completions = complete_tables(engine);
            for node in registry.nodes() {
                if !completions.iter().any(|c| c.text == node.name) {
                    completions.push(Completion {
                        text: node.name.clone(),
                        kind: CompletionKind::Table,
                    });
                }
            }
            completions
        }
        _ => {
            // Complete columns of the referenced table.
            let table = parts.get(1).unwrap_or(&"");
            complete_columns(table, engine)
        }
    }
}

fn complete_into_args(rest: &str) -> Vec<Completion> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.is_empty() {
        return vec![]; // user picks table name
    }
    // After table name, suggest flags.
    vec![
        Completion {
            text: "-file".into(),
            kind: CompletionKind::Command,
        },
        Completion {
            text: "-source".into(),
            kind: CompletionKind::Command,
        },
        Completion {
            text: "-csv".into(),
            kind: CompletionKind::Command,
        },
        Completion {
            text: "-json".into(),
            kind: CompletionKind::Command,
        },
        Completion {
            text: "-regex".into(),
            kind: CompletionKind::Command,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_completion() {
        let engine = Engine::open_memory().unwrap();
        let registry = Registry::new();

        let results = complete("sq", &engine, &registry);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "sql");

        let results = complete("p", &engine, &registry);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "plot");
    }

    #[test]
    fn test_table_completion() {
        let engine = Engine::open_memory().unwrap();
        engine.query("CREATE TABLE auth (x INT)").unwrap();
        engine.query("CREATE TABLE flows (y INT)").unwrap();
        let registry = Registry::new();

        let results = complete("sql ", &engine, &registry);
        assert!(results.iter().any(|c| c.text == "auth"));
        assert!(results.iter().any(|c| c.text == "flows"));
    }

    #[test]
    fn test_column_completion() {
        let engine = Engine::open_memory().unwrap();
        engine
            .query("CREATE TABLE auth (username TEXT, src_ip TEXT, ts TEXT)")
            .unwrap();
        let registry = Registry::new();

        let results = complete("sql FROM auth ", &engine, &registry);
        assert!(results.iter().any(|c| c.text == "username"));
        assert!(results.iter().any(|c| c.text == "src_ip"));
    }
}
