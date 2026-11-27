// SPDX-License-Identifier: MIT
//! Changes go through the `buckets` CLI with `--json`.
//!
//! Commit, lock, finalize and the rest live in the CLI binary, not the
//! library, and they carry the rules that keep a repository safe (lock checks,
//! transactions, atomic writes). Calling the CLI means the app can never
//! disagree with it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::model::CliInfo;

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "buckets.exe"
    } else {
        "buckets"
    }
}

/// Find the CLI: the configured path, next to this app, on PATH, then in
/// the usual install folders.
pub fn locate(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(exe_name()));
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&paths).map(|d| d.join(exe_name())));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".cargo").join("bin").join(exe_name()));
    }
    candidates.push(PathBuf::from("/usr/local/bin").join(exe_name()));
    candidates.push(PathBuf::from("/opt/homebrew/bin").join(exe_name()));
    candidates.into_iter().find(|p| p.is_file())
}

pub fn info(configured: Option<&Path>) -> CliInfo {
    let path = locate(configured);
    let version = path.as_ref().and_then(|p| {
        Command::new(p)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });
    CliInfo {
        path: path.map(|p| p.display().to_string()),
        version,
    }
}

/// Run `buckets <args> --json` in `cwd`. Returns the parsed JSON (or the raw
/// text, for the few commands that print plain text).
pub fn run(
    cli: &Path,
    cwd: &Path,
    args: &[String],
    similarity_check: bool,
) -> Result<Value, String> {
    let mut command = Command::new(cli);
    command
        .args(args)
        .arg("--json")
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !similarity_check {
        command.env("BUCKETS_NO_EMBEDDINGS", "1");
    }
    let output = command
        .output()
        .map_err(|e| format!("Could not run {}: {}", cli.display(), e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let message = if stderr.is_empty() { stdout } else { stderr };
        return Err(clean_error(&message));
    }
    Ok(serde_json::from_str(&stdout).unwrap_or(Value::String(stdout)))
}

/// Strip log prefixes and ANSI colour codes from CLI errors.
fn clean_error(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip an escape sequence like ESC[1;31m.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    let lines: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        "The buckets CLI failed without a message".to_string()
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::clean_error;

    #[test]
    fn strips_ansi_and_blank_lines() {
        assert_eq!(
            clean_error("\u{1b}[31mError:\u{1b}[0m locked\n\n  by maya \n"),
            "Error: locked\nby maya"
        );
        assert_eq!(clean_error(""), "The buckets CLI failed without a message");
    }
}
