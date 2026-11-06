//! End-to-end tests: run the real binary in a temporary folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Fresh, empty folder for one test, with the given `.env.example` and `.env` (if any).
fn workdir(name: &str, example: Option<&str>, env: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    if let Some(example) = example {
        fs::write(dir.join(".env.example"), example).unwrap();
    }
    if let Some(env) = env {
        fs::write(dir.join(".env"), env).unwrap();
    }
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_env-sync"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn read_env(dir: &Path) -> String {
    fs::read_to_string(dir.join(".env")).unwrap()
}

#[test]
fn creates_env_from_template() {
    let dir = workdir("create", Some("# db\nHOST=localhost\n"), None);
    assert!(run(&dir, &[]).status.success());
    assert_eq!(read_env(&dir), "# db\nHOST=localhost\n");
}

#[test]
fn keeps_values_and_moves_deprecated_keys() {
    let dir = workdir("merge", Some("A=1\nB=2\n"), Some("OLD=x\nB=kept\n"));
    assert!(run(&dir, &[]).status.success());
    let env = read_env(&dir);
    assert!(env.starts_with("A=1\nB=kept\n\n# ==="));
    assert!(env.ends_with("\nOLD=x\n"));
}

#[test]
fn second_run_changes_nothing() {
    let dir = workdir("idempotent", Some("A=1\nS=# RandomHex\n"), Some("OLD=x\n"));
    assert!(run(&dir, &[]).status.success());
    let first = read_env(&dir);
    assert!(run(&dir, &[]).status.success());
    assert_eq!(read_env(&dir), first);
}

#[test]
fn missing_template_fails_without_creating_env() {
    let dir = workdir("no-template", None, None);
    let out = run(&dir, &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains(".env.example"));
    assert!(!dir.join(".env").exists());
}

#[test]
fn failing_command_leaves_env_untouched() {
    let dir = workdir("failing", Some("S=# Cmd exit 1\n"), Some("A=1\n"));
    let out = run(&dir, &[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("exit 1"));
    assert_eq!(read_env(&dir), "A=1\n");
}

#[test]
fn rejects_arguments() {
    let dir = workdir("args", Some("A=1\n"), None);
    assert!(!run(&dir, &["extra"]).status.success());
    assert!(!dir.join(".env").exists());
}

#[test]
fn unreadable_env_is_not_overwritten() {
    let dir = workdir("unreadable", Some("A=1\n"), None);
    fs::write(dir.join(".env"), [0xff, 0xfe]).unwrap();
    assert!(!run(&dir, &[]).status.success());
    assert_eq!(fs::read(dir.join(".env")).unwrap(), [0xff, 0xfe]);
}

#[test]
fn leaves_no_temporary_file() {
    let dir = workdir("tmp", Some("A=1\n"), None);
    fs::write(dir.join(".env.env-sync.tmp"), "stale").unwrap();
    fs::write(dir.join(".env.tmp"), "mine").unwrap();
    assert!(run(&dir, &[]).status.success());
    assert!(!dir.join(".env.env-sync.tmp").exists());
    assert_eq!(fs::read_to_string(dir.join(".env.tmp")).unwrap(), "mine");
}

#[cfg(unix)]
#[test]
fn env_is_readable_only_by_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = workdir("mode", Some("A=1\n"), Some("A=2\n"));
    assert!(run(&dir, &[]).status.success());
    let mode = fs::metadata(dir.join(".env")).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[cfg(unix)]
#[test]
fn writes_through_a_symlinked_env() {
    let dir = workdir("symlink", Some("A=1\n"), None);
    fs::write(dir.join("real.env"), "A=2\n").unwrap();
    std::os::unix::fs::symlink("real.env", dir.join(".env")).unwrap();
    assert!(run(&dir, &[]).status.success());
    assert!(fs::symlink_metadata(dir.join(".env")).unwrap().is_symlink());
    assert_eq!(fs::read_to_string(dir.join("real.env")).unwrap(), "A=2\n");
}

#[test]
fn unknown_hash_value_is_copied() {
    let dir = workdir("hash", Some("P=# the port\n"), None);
    assert!(run(&dir, &[]).status.success());
    assert_eq!(read_env(&dir), "P=# the port\n");
}

#[cfg(unix)]
#[test]
fn dangling_symlink_fails() {
    let dir = workdir("dangling", Some("A=1\n"), None);
    std::os::unix::fs::symlink("missing.env", dir.join(".env")).unwrap();
    assert!(!run(&dir, &[]).status.success());
    assert!(fs::symlink_metadata(dir.join(".env")).unwrap().is_symlink());
}

#[test]
fn custom_env_and_template() {
    let dir = workdir("custom", None, None);
    fs::write(dir.join("prod.example"), "A=1\n").unwrap();
    fs::write(dir.join(".env.prod"), "OLD=x\n").unwrap();
    assert!(
        run(&dir, &["-e", ".env.prod", "--template", "prod.example"])
            .status
            .success()
    );
    let env = fs::read_to_string(dir.join(".env.prod")).unwrap();
    assert!(env.starts_with("A=1\n"));
    assert!(env.contains("are in your .env.prod"));
    assert!(env.contains("no longer tracked in prod.example"));
    assert!(!dir.join(".env").exists());
}

#[test]
fn refuses_to_overwrite_the_template() {
    let dir = workdir("same", Some("A=1\n"), None);
    assert!(!run(&dir, &["--env", ".env.example"]).status.success());
    assert_eq!(
        fs::read_to_string(dir.join(".env.example")).unwrap(),
        "A=1\n"
    );
}

#[test]
fn prints_help() {
    let dir = workdir("help", None, None);
    let out = run(&dir, &["--help"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("--template"));
}
