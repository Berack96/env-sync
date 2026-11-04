//! Syncs `.env` with `.env.example`: keeps the template's order, comments and blank lines,
//! preserves values already set, generates missing defaults and moves keys no longer in the
//! template to the bottom.
//!
//! Usage: `env-sync [--env <file>] [--template <file>]`, see [`USAGE`].

use std::collections::HashSet;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const ENV_FILE: &str = ".env";
const EXAMPLE_FILE: &str = ".env.example";
const USAGE: &str = "\
Usage: env-sync [--env <file>] [--template <file>]

Syncs the env file with its template, in the current folder.

Options:
  -e, --env <file>       env file to sync, read and rewritten [default: .env]
  -t, --template <file>  template to sync from [default: .env.example]
  -h, --help             print this help";

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
    /// `None` if `spec` does not start with a known command name.
    fn parse(spec: &str) -> Result<Option<Self>, String> {
        let (name, args) = spec.split_once(char::is_whitespace).unwrap_or((spec, ""));
        let args = args.trim();
        match (name, args.is_empty()) {
            ("RandomHex", true) => Ok(Some(Self::RandomHex)),
            ("RandomBase64", true) => Ok(Some(Self::RandomBase64)),
            ("Uuid", true) => Ok(Some(Self::Uuid)),
            ("Cmd", false) => Ok(Some(Self::Cmd(args.to_string()))),
            ("RandomHex" | "RandomBase64" | "Uuid", false) => {
                Err(format!("{name} takes no arguments"))
            }
            ("Cmd", true) => Err("Cmd needs a command".to_string()),
            _ => Ok(None),
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
                let h = hex(&b);
                Ok(format!(
                    "{}-{}-{}-{}-{}",
                    &h[..8],
                    &h[8..12],
                    &h[12..16],
                    &h[16..20],
                    &h[20..]
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

/// Evaluates the default: `# Command` runs a known command, any other value (comments too) is kept as is.
fn eval_default(value: &str) -> Result<String, String> {
    let command = match value.strip_prefix('#') {
        Some(spec) => CommandEnvs::parse(spec.trim())?,
        None => None,
    };
    command.map_or_else(|| Ok(value.to_string()), |c| c.eval())
}

/// Content of the new env file, from the template (named `template`) and the current env file (empty if missing).
fn sync(example: &str, current: &str, template: &str) -> Result<String, String> {
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
        out.push(format!("# but are no longer tracked in {template}"));
        out.push("# ===================================================".to_string());
        out.extend(deprecated.iter().map(|l| l.to_string()));
    }

    out.push(String::new());
    Ok(out.join("\n"))
}

/// `(env, template)` from the command line, `None` if help was asked.
fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<(String, String)>, String> {
    let (mut env, mut template) = (ENV_FILE.to_string(), EXAMPLE_FILE.to_string());
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "-e" | "--env" => &mut env,
            "-t" | "--template" => &mut template,
            "-h" | "--help" => return Ok(None),
            _ => return Err(format!("unknown argument: {arg}\n\n{USAGE}")),
        };
        *slot = args
            .next()
            .ok_or_else(|| format!("{arg} needs a file\n\n{USAGE}"))?;
    }
    Ok(Some((env, template)))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let Some((env, template)) = parse_args(std::env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(());
    };
    let example =
        fs::read_to_string(&template).map_err(|e| format!("failed to read {template}: {e}"))?;
    if fs::canonicalize(&env).ok() == fs::canonicalize(&template).ok() {
        return Err(format!("{env} is the template itself"));
    }
    let current = match fs::read_to_string(&env) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        r => r.map_err(|e| format!("failed to read {env}: {e}"))?,
    };
    let content = sync(&example, &current, &template).map_err(|e| format!("{template}: {e}"))?;
    write_env(&env, &content).map_err(|e| format!("failed to write {env}: {e}"))
}

/// Writes a temporary file and renames it over `env` (or the file it links to), so a failed write
/// never leaves it half written. On Unix the file is readable only by its owner.
fn write_env(env: &str, content: &str) -> std::io::Result<()> {
    // A dangling symlink is NotFound too: it must fail, not be replaced by a regular file.
    let target = match fs::canonicalize(env) {
        Err(e) if fs::symlink_metadata(env).is_err() => {
            if e.kind() == std::io::ErrorKind::NotFound {
                env.into()
            } else {
                return Err(e);
            }
        }
        r => r?,
    };
    let mut tmp = target.clone().into_os_string();
    tmp.push(".env-sync.tmp");
    let _ = fs::remove_file(&tmp);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = options
        .open(&tmp)
        .and_then(|mut f| f.write_all(content.as_bytes()).and_then(|_| f.sync_all()))
        .and_then(|_| fs::rename(&tmp, &target));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync(example: &str, current: &str) -> Result<String, String> {
        super::sync(example, current, EXAMPLE_FILE)
    }

    fn args(list: &[&str]) -> Result<Option<(String, String)>, String> {
        parse_args(list.iter().map(|a| a.to_string()))
    }

    #[test]
    fn parses_arguments() {
        let pair = |e: &str, t: &str| Ok(Some((e.to_string(), t.to_string())));
        assert_eq!(args(&[]), pair(".env", ".env.example"));
        assert_eq!(
            args(&["-e", ".env.prod"]),
            pair(".env.prod", ".env.example")
        );
        assert_eq!(args(&["--template", "t", "--env", "e"]), pair("e", "t"));
        assert_eq!(args(&["--help"]), Ok(None));
        assert!(args(&["--env"]).is_err());
        assert!(args(&["extra"]).is_err());
    }

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
    fn other_hash_values_kept() {
        let example = "S=# Nope\nC=#fff\nP=# the server port\n";
        assert_eq!(sync(example, "").unwrap(), example);
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
        assert!(sync("V=# Cmd\n", "").is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn cmd_rejects_multiline_output() {
        assert!(sync("V=# Cmd echo a; echo b\n", "").is_err());
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
