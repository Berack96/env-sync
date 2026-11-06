//! Syncs `.env` with `.env.example`: keeps the template's order, comments and blank lines,
//! preserves values already set, generates missing defaults and moves every other line of the
//! env file (deprecated or repeated keys, the user's own comments) to the bottom.
//!
//! Usage: `env-sync [--env <file>] [--template <file>] [--check]`, see [`USAGE`].

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const ENV_FILE: &str = ".env";
const EXAMPLE_FILE: &str = ".env.example";
const DEPRECATED_TITLE: &str = "# ⚠️  DEPRECATED OR LOCAL VARIABLES";
const USAGE: &str = "\
Usage: env-sync [--env <file>] [--template <file>] [--check]

Syncs the env file with its template, in the current folder.

Options:
  -e, --env <file>       env file to sync, read and rewritten [default: .env]
  -t, --template <file>  template to sync from [default: .env.example]
  -c, --check            print the keys a sync would add (+) or deprecate (-),
                         without writing the env file or running commands
  -h, --help             print this help

Exit status: 0 on success, 1 if --check finds changes, 2 on error.";

#[derive(Debug, PartialEq)]
enum Mode {
    Sync,
    Check,
}

/// If the line is a `KEY=value` assignment (spaces around the equals sign and an `export` prefix
/// allowed) returns (key, value).
fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    if let Some(rest) = exported(line)
        && let Some(assignment) = parse_assignment(rest)
    {
        return Some(assignment);
    }
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let mut chars = key.chars();
    let valid = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some((key, value.trim()))
}

/// The rest of `line` after an `export ` prefix, if it has one.
fn exported(line: &str) -> Option<&str> {
    line.trim_start()
        .strip_prefix("export")
        .filter(|rest| rest.starts_with(char::is_whitespace))
}

/// Lines of `text`, with a quoted value that spans several lines joined into one entry.
/// shortcut: escaped quotes (`\"`) are not recognized, handle them if a real file needs it.
fn entries(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        let quote = parse_assignment(line).and_then(|(_, v)| {
            v.chars()
                .next()
                .filter(|q| ['"', '\''].contains(q))
                .zip(Some(v))
        });
        // An unterminated quote stays a single line, so it cannot swallow the rest of the file.
        let end = match quote {
            Some((q, value)) if !value[1..].contains(q) => lines[i..]
                .iter()
                .position(|l| l.contains(q))
                .map(|p| i + p + 1),
            _ => None,
        };
        let end = end.unwrap_or(i);
        out.push(lines[i - 1..end].join("\n"));
        i = end;
    }
    out
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

/// Content of the new env file (named `env`), from the template (named `template`) and the
/// current env file (empty if missing). Without `run_commands` missing keys get the raw default.
fn sync(
    example: &str,
    current: &str,
    env: &str,
    template: &str,
    run_commands: bool,
) -> Result<String, String> {
    let header = [
        "# ===================================================".to_string(),
        DEPRECATED_TITLE.to_string(),
        format!("# The following variables are in your {env}"),
        format!("# but are no longer tracked in {template}"),
        "# ===================================================".to_string(),
    ];
    let current = entries(current);
    let mut used = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for entry in entries(example) {
        let Some((key, default)) = parse_assignment(&entry) else {
            out.push(entry);
            continue;
        };
        let existing = (0..current.len()).find(|i| {
            !used.contains(i) && parse_assignment(&current[*i]).is_some_and(|(k, _)| k == key)
        });
        out.push(match existing {
            Some(i) => {
                used.insert(i);
                current[i].clone()
            }
            None => format!(
                "{}{key}={}",
                if exported(&entry).is_some() {
                    "export "
                } else {
                    ""
                },
                if run_commands {
                    eval_default(default).map_err(|e| format!("{key}: {e}"))?
                } else {
                    default.to_string()
                }
            ),
        });
    }

    // Every other line of the env file is kept at the bottom: deprecated or repeated keys, and
    // the user's own comments (not a template line already in place, nor a warning header).
    let mut template_lines: HashMap<&str, usize> = HashMap::new();
    for line in example.lines() {
        *template_lines.entry(line).or_default() += 1;
    }
    let is_header = |e: &str| {
        header.iter().any(|h| h == e)
            || e.starts_with("# The following variables are in your ")
            || e.starts_with("# but are no longer tracked in ")
    };
    let mut rest: Vec<&String> = Vec::new();
    for (i, e) in current.iter().enumerate() {
        if used.contains(&i) || e.trim().is_empty() {
            continue;
        }
        if parse_assignment(e).is_none() {
            if let Some(n) = template_lines.get_mut(e.as_str()).filter(|n| **n > 0) {
                *n -= 1;
                continue;
            }
            if is_header(e) {
                continue;
            }
        }
        rest.push(e);
    }
    if !rest.is_empty() {
        out.push(String::new());
        out.extend(header);
        out.extend(rest.into_iter().cloned());
    }

    out.push(String::new());
    Ok(out.join("\n"))
}

/// `(key, value)` of each assignment in `text`, first occurrence only. With `active_only` the
/// deprecated section is skipped.
fn assignments(text: &str, active_only: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in entries(text) {
        if active_only && entry == DEPRECATED_TITLE {
            break;
        }
        if let Some((key, value)) = parse_assignment(&entry)
            && !out.iter().any(|(k, _)| k == key)
        {
            out.push((key.to_string(), value.to_string()));
        }
    }
    out
}

/// Keys that differ from `old` to `new`: `-` only in `old`, `~` changed value, `+` only in `new`.
fn diff(old: &[(String, String)], new: &[(String, String)]) -> Vec<String> {
    let find = |list: &[(String, String)], key: &str| {
        list.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    let mut out = Vec::new();
    for (key, value) in old {
        match find(new, key) {
            None => out.push(format!("-{key}")),
            Some(v) if &v != value => out.push(format!("~{key}")),
            _ => {}
        }
    }
    for (key, _) in new {
        if find(old, key).is_none() {
            out.push(format!("+{key}"));
        }
    }
    out
}

/// `(env, template, mode)` from the command line, `None` if help was asked.
fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> Result<Option<(String, String, Mode)>, String> {
    let (mut env, mut template) = (ENV_FILE.to_string(), EXAMPLE_FILE.to_string());
    let mut check = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "-e" | "--env" => &mut env,
            "-t" | "--template" => &mut template,
            "-c" | "--check" => {
                check = true;
                continue;
            }
            "-h" | "--help" => return Ok(None),
            _ => return Err(format!("unknown argument: {arg}\n\n{USAGE}")),
        };
        *slot = args
            .next()
            .filter(|v| !v.starts_with('-'))
            .ok_or_else(|| format!("{arg} needs a file\n\n{USAGE}"))?;
    }
    let mode = if check { Mode::Check } else { Mode::Sync };
    Ok(Some((env, template, mode)))
}

fn main() {
    match run() {
        Ok(changed) => std::process::exit(i32::from(changed)),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}

/// `true` if a check found changes.
fn run() -> Result<bool, String> {
    let Some((env, template, mode)) = parse_args(std::env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(false);
    };
    let example =
        fs::read_to_string(&template).map_err(|e| format!("failed to read {template}: {e}"))?;
    if fs::canonicalize(&env).ok() == Some(fs::canonicalize(&template).map_err(|e| e.to_string())?)
    {
        return Err(format!("{env} is the template itself"));
    }
    let current = match fs::read_to_string(&env) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        r => r.map_err(|e| format!("failed to read {env}: {e}"))?,
    };
    let content = sync(&example, &current, &env, &template, mode == Mode::Sync)
        .map_err(|e| format!("{template}: {e}"))?;
    if mode == Mode::Check {
        for change in diff(&assignments(&current, true), &assignments(&content, true)) {
            println!("{change}");
        }
        return Ok(content != current);
    }
    write_env(&env, &content).map_err(|e| format!("failed to write {env}: {e}"))?;
    Ok(false)
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
        super::sync(example, current, ENV_FILE, EXAMPLE_FILE, true)
    }

    fn args(list: &[&str]) -> Result<Option<(String, String, Mode)>, String> {
        parse_args(list.iter().map(|a| a.to_string()))
    }

    #[test]
    fn parses_arguments() {
        let pair = |e: &str, t: &str| Ok(Some((e.to_string(), t.to_string(), Mode::Sync)));
        assert_eq!(args(&[]), pair(".env", ".env.example"));
        assert_eq!(
            args(&["-e", ".env.prod"]),
            pair(".env.prod", ".env.example")
        );
        assert_eq!(args(&["--template", "t", "--env", "e"]), pair("e", "t"));
        assert_eq!(args(&["--help"]), Ok(None));
        assert_eq!(
            args(&["--check"]),
            Ok(Some((ENV_FILE.into(), EXAMPLE_FILE.into(), Mode::Check)))
        );
        assert!(args(&["--env"]).is_err());
        assert!(args(&["-e", "-t", "x"]).is_err());
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
    fn keeps_export_prefix() {
        assert_eq!(sync("A=1\n", "export A=9\n").unwrap(), "export A=9\n");
    }

    #[test]
    fn keeps_multiline_values() {
        let current = "K=\"-----BEGIN\nline2\n-----END\"\nB=2\n";
        assert_eq!(sync("K=\nB=1\n", current).unwrap(), current);
    }

    #[test]
    fn repeated_keys_and_own_comments_at_bottom() {
        let out = sync("# t\nB=1\n", "# t\nB=7\nB=8\n# mine\n").unwrap();
        assert!(out.starts_with("# t\nB=7\n\n# ==="));
        assert!(out.ends_with("===\nB=8\n# mine\n"));
        assert_eq!(sync("# t\nB=1\n", &out).unwrap(), out);
    }

    #[test]
    fn unterminated_quote_is_one_line() {
        let example = "A=1\nB=\"\nC=3\nexport D=4\n";
        assert_eq!(
            sync(example, "C=9\n").unwrap(),
            "A=1\nB=\"\nC=9\nexport D=4\n"
        );
    }

    #[test]
    fn keeps_multiline_values_with_base64_padding() {
        let current = "K=\"-----BEGIN\nMIIBabc=\n-----END\"\n";
        assert_eq!(sync("K=\n", current).unwrap(), current);
    }

    #[test]
    fn repeated_template_key_is_stable() {
        let out = sync("A=1\nA=2\n", "A=9\n").unwrap();
        assert_eq!(out, "A=9\nA=2\n");
        assert_eq!(sync("A=1\nA=2\n", &out).unwrap(), out);
    }

    #[test]
    fn repeated_own_comments_kept() {
        let out = sync("# t\nA=1\n", "# t\nA=1\n# t\n").unwrap();
        assert!(out.ends_with("===\n# t\n"));
    }

    #[test]
    fn old_header_dropped_after_template_rename() {
        let old = super::sync("A=1\n", "OLD=x\n", ENV_FILE, "old.example", true).unwrap();
        let out = sync("A=1\n", &old).unwrap();
        assert_eq!(out.matches("# ===").count(), 2);
        assert!(!out.contains("old.example"));
    }

    #[test]
    fn check_runs_no_command() {
        let out = super::sync("V=# Cmd exit 1\n", "", ENV_FILE, EXAMPLE_FILE, false);
        assert_eq!(out.unwrap(), "V=# Cmd exit 1\n");
    }

    #[test]
    fn diffs_keys() {
        let old = assignments("A=1\nB=2\nK=\"x\ny\"\nD=0\n", false);
        let new = assignments("export B=3\nK=\"x\nz\"\nC=4\nA=1\nA=9\n", false);
        assert_eq!(diff(&old, &new), ["~B", "~K", "-D", "+C"]);
    }

    #[test]
    fn active_assignments_skip_deprecated() {
        let out = sync("A=1\n", "OLD=x\n").unwrap();
        assert_eq!(assignments(&out, true), [("A".into(), "1".into())]);
        assert_eq!(assignments(&out, false).len(), 2);
    }

    #[test]
    fn invalid_keys() {
        assert_eq!(parse_assignment("1A=x"), None);
        assert_eq!(parse_assignment("# A=x"), None);
        assert_eq!(parse_assignment(" A = x "), Some(("A", "x")));
        assert_eq!(parse_assignment("export A=x"), Some(("A", "x")));
        assert_eq!(parse_assignment("exportA=x"), Some(("exportA", "x")));
        assert_eq!(parse_assignment("export = x"), Some(("export", "x")));
    }
}
