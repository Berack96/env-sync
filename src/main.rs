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

/// Commands that generate a key's default, written in `.env.example` as `KEY=# Command [args]`.
enum CommandEnvs {
    /// 32 random bytes in hex.
    RandomHex,
    /// 32 random bytes in base64.
    RandomBase64,
    /// Random UUID (version 4).
    Uuid,
    /// Stdout of a shell command.
    Cmd(String),
}

impl CommandEnvs {
    fn parse(spec: &str) -> Result<Self, String> {
        let (name, args) = spec.split_once(char::is_whitespace).unwrap_or((spec, ""));
        let args = args.trim();
        match (name, args.is_empty()) {
            ("RandomHex", true) => Ok(Self::RandomHex),
            ("RandomBase64", true) => Ok(Self::RandomBase64),
            ("Uuid", true) => Ok(Self::Uuid),
            ("Cmd", false) => Ok(Self::Cmd(args.to_string())),
            ("RandomHex" | "RandomBase64" | "Uuid", false) => {
                Err(format!("{name} takes no arguments in {EXAMPLE_FILE}"))
            }
            ("Cmd", true) => Err(format!("Cmd needs a command in {EXAMPLE_FILE}")),
            _ => Err(format!("unknown command in {EXAMPLE_FILE}: {name}")),
        }
    }

    fn eval(&self) -> Result<String, String> {
        match self {
            Self::RandomHex => Ok(hex(&random_bytes::<32>())),
            Self::RandomBase64 => Ok(base64(&random_bytes::<32>())),
            Self::Uuid => {
                let mut b = random_bytes::<16>();
                b[6] = (b[6] & 0x0f) | 0x40;
                b[8] = (b[8] & 0x3f) | 0x80;
                let hex = hex(&b);
                Ok(format!(
                    "{}-{}-{}-{}-{}",
                    &hex[..8],
                    &hex[8..12],
                    &hex[12..16],
                    &hex[16..20],
                    &hex[20..]
                ))
            }
            Self::Cmd(cmd) => run_shell(cmd),
        }
    }
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("system random generator not available");
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Standard base64, with padding.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().fold(0u32, |n, &b| (n << 8) | b as u32) << (8 * (3 - chunk.len()));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                ALPHABET[(n >> (18 - 6 * i)) as usize & 63] as char
            } else {
                '='
            });
        }
    }
    out
}

/// Runs `cmd` in the system shell and returns its stdout without the trailing newline.
fn run_shell(cmd: &str) -> Result<String, String> {
    let (shell, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let mut command = std::process::Command::new(shell);
    command.arg(flag);
    // `cmd /C` does not follow the quoting rules `arg` applies on Windows.
    #[cfg(windows)]
    std::os::windows::process::CommandExt::raw_arg(&mut command, cmd);
    #[cfg(not(windows))]
    command.arg(cmd);
    let out = command
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("failed to run `{cmd}`: {e}"))?;
    if !out.status.success() {
        return Err(format!("`{cmd}` failed: {}", out.status));
    }
    let value =
        String::from_utf8(out.stdout).map_err(|_| format!("`{cmd}` printed non UTF-8 output"))?;
    let value = value.trim_end_matches(['\r', '\n']);
    if value.contains(['\r', '\n']) {
        return Err(format!("`{cmd}` printed more than one line"));
    }
    Ok(value.to_string())
}

/// Evaluates the default: `# Command` runs the command, any other value is kept as is.
fn eval_default(value: &str) -> Result<String, String> {
    match value.strip_prefix('#') {
        Some(spec) => CommandEnvs::parse(spec.trim())?.eval(),
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
            None => format!(
                "{key}={}",
                eval_default(default).map_err(|e| format!("{key}: {e}"))?
            ),
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
    fn random_base64_and_uuid_commands() {
        let out = sync("B=# RandomBase64\nU=# Uuid\n", "").unwrap();
        let mut lines = out.lines();
        let b64 = lines.next().unwrap().strip_prefix("B=").unwrap();
        assert_eq!(b64.len(), 44);
        assert!(b64.ends_with('='));
        let uuid = lines.next().unwrap().strip_prefix("U=").unwrap();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            [8, 4, 4, 4, 12]
        );
        assert!(parts[2].starts_with('4'));
        assert!(parts[3].starts_with(['8', '9', 'a', 'b']));
    }

    #[test]
    fn base64_known_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn cmd_command() {
        assert_eq!(sync("V=# Cmd echo hello\n", "").unwrap(), "V=hello\n");
        assert_eq!(
            sync("V=# Cmd echo hello\n", "V=kept\n").unwrap(),
            "V=kept\n"
        );
        assert!(sync("V=# Cmd exit 1\n", "").is_err());
        assert!(sync("V=# Cmd echo a; echo b\n", "").is_err());
        assert!(sync("V=# Cmd\n", "").is_err());
    }

    #[test]
    fn preset_rejects_arguments() {
        assert!(sync("S=# RandomHex 16\n", "").is_err());
    }

    #[test]
    fn invalid_keys() {
        assert_eq!(parse_assignment("1A=x"), None);
        assert_eq!(parse_assignment("# A=x"), None);
        assert_eq!(parse_assignment(" A = x "), Some(("A", "x")));
    }
}
