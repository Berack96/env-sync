//! Syncs `.env` with `.env.example`: keeps the template's order, comments and blank lines,
//! preserves values already set, generates missing defaults and moves keys no longer in the
//! template to the bottom.
//!
//! Usage: `env-sync`, in the folder with `.env.example`.

use std::collections::HashSet;
use std::fs;

const ENV_FILE: &str = ".env";
const EXAMPLE_FILE: &str = ".env.example";

/// If the line is a `KEY=value` assignment (spaces around the equals sign allowed) returns (key, value).
fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let mut chars = key.chars();
    let valid = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some((key, value.trim()))
}

/// Commands that generate a key's default, written in `.env.example` as `KEY=# Command`.
enum CommandEnvs {
    /// 32 random bytes in hex.
    RandomHex,
}

impl CommandEnvs {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "RandomHex" => Ok(Self::RandomHex),
            _ => Err(format!("unknown command in {EXAMPLE_FILE}: {name}")),
        }
    }

    fn eval(&self) -> String {
        match self {
            Self::RandomHex => {
                let mut bytes = [0u8; 32];
                getrandom::fill(&mut bytes).expect("system random generator not available");
                bytes.iter().map(|b| format!("{b:02x}")).collect()
            }
        }
    }
}

/// Evaluates the default: `# Command` runs the command, any other value is kept as is.
fn eval_default(value: &str) -> Result<String, String> {
    match value.strip_prefix('#') {
        Some(name) => CommandEnvs::parse(name.trim()).map(|c| c.eval()),
        None => Ok(value.to_string()),
    }
}

/// Content of the new `.env`, from the template and the current `.env` (empty if missing).
fn sync(example: &str, current: &str) -> Result<String, String> {
    let mut out: Vec<String> = Vec::new();
    let mut known = HashSet::new();
    for line in example.lines() {
        let Some((key, default)) = parse_assignment(line) else {
            out.push(line.to_string());
            continue;
        };
        known.insert(key);
        let existing = current
            .lines()
            .find(|l| parse_assignment(l).is_some_and(|(k, _)| k == key));
        out.push(match existing {
            Some(l) => l.to_string(),
            None => format!("{key}={}", eval_default(default)?),
        });
    }

    let deprecated: Vec<&str> = current
        .lines()
        .filter(|l| parse_assignment(l).is_some_and(|(k, _)| !known.contains(k)))
        .collect();
    if !deprecated.is_empty() {
        out.push(String::new());
        out.push("# ===================================================".to_string());
        out.push("# ⚠️  DEPRECATED OR LOCAL VARIABLES".to_string());
        out.push("# The following variables are in your .env".to_string());
        out.push(format!("# but are no longer tracked in {EXAMPLE_FILE}"));
        out.push("# ===================================================".to_string());
        out.extend(deprecated.iter().map(|l| l.to_string()));
    }

    out.push(String::new());
    Ok(out.join("\n"))
}

fn main() -> Result<(), String> {
    if std::env::args().len() > 1 {
        return Err(format!(
            "takes no arguments: syncs {ENV_FILE} with {EXAMPLE_FILE} in the current folder"
        ));
    }
    let example = fs::read_to_string(EXAMPLE_FILE)
        .map_err(|e| format!("failed to read {EXAMPLE_FILE}: {e}"))?;
    let current = fs::read_to_string(ENV_FILE).unwrap_or_default();
    fs::write(ENV_FILE, sync(&example, &current)?)
        .map_err(|e| format!("failed to write {ENV_FILE}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_template_and_values() {
        let example = "# comment\nA=1\n\nB = 2\n";
        let out = sync(example, "B=x\nA=y\n").unwrap();
        assert_eq!(out, "# comment\nA=y\n\nB=x\n");
    }

    #[test]
    fn defaults_for_missing() {
        assert_eq!(sync("A=1\n", "").unwrap(), "A=1\n");
    }

    #[test]
    fn deprecated_at_bottom() {
        let out = sync("A=1\n", "A=2\nOLD=3\n").unwrap();
        assert!(out.starts_with("A=2\n\n# ==="));
        assert!(out.ends_with("OLD=3\n"));
    }

    #[test]
    fn random_hex_command() {
        let out = sync("S=# RandomHex\n", "").unwrap();
        let value = out.trim_end().strip_prefix("S=").unwrap();
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(sync("S=# RandomHex\n", &out).unwrap(), out);
    }

    #[test]
    fn unknown_command() {
        assert!(sync("S=# Nope\n", "").is_err());
    }

    #[test]
    fn invalid_keys() {
        assert_eq!(parse_assignment("1A=x"), None);
        assert_eq!(parse_assignment("# A=x"), None);
        assert_eq!(parse_assignment(" A = x "), Some(("A", "x")));
    }
}
