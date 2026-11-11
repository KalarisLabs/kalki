//! `cargo xtask` — repo automation. std-only; no external crates by design.
//!
//! Usage: `cargo xtask ci` (full local gate, mirrors `.github/workflows/ci.yml`)
//!      | `cargo xtask lint` (repo rules cargo cannot express)

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// AGENTS.md rule 22: hard cap on Rust source file length.
const FILE_LINE_CAP: usize = 500;

/// Files already over the cap when the lint landed. Each entry is a ratchet:
/// the file may shrink, never grow. Remove the entry once the file is split.
const SIZE_DEBT: &[(&str, usize)] = &[
    ("crates/kalki-core/src/session.rs", 533),
    ("crates/kalki-core/src/tools.rs", 791),
    ("crates/kalki-core/tests/e2e_mock.rs", 662),
];

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("ci") => {
            if !run("cargo", &["fmt", "--all", "--", "--check"], &[]) {
                return ExitCode::FAILURE;
            }
            if lint() == ExitCode::FAILURE {
                return ExitCode::FAILURE;
            }
            let clippy = [
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ];
            // Same flag as the CI doc job: a broken intra-doc link fails here, not on GitHub.
            let doc_env = [("RUSTDOCFLAGS", "-D warnings")];
            let ok = run("cargo", &clippy, &[])
                && run("cargo", &["test", "--workspace", "--locked"], &[])
                && run(
                    "cargo",
                    &["doc", "--workspace", "--no-deps", "--locked"],
                    &doc_env,
                );
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("lint") => lint(),
        _ => {
            eprintln!("usage: cargo xtask ci|lint");
            ExitCode::from(2)
        }
    }
}

fn run(program: &str, args: &[&str], env: &[(&str, &str)]) -> bool {
    eprintln!("==> {program} {}", args.join(" "));
    match Command::new(program)
        .args(args)
        .envs(env.iter().copied())
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(_) => {
            eprintln!("step failed: {program} {}", args.join(" "));
            false
        }
        Err(e) => {
            eprintln!("failed to spawn {program}: {e}");
            false
        }
    }
}

/// Repo rules with remediation text, so an agent that trips one knows the fix.
fn lint() -> ExitCode {
    eprintln!("==> xtask lint (file-size ratchet)");
    let mut files = Vec::new();
    for root in ["crates", "xtask"] {
        collect_rust_files(Path::new(root), &mut files);
    }
    let mut failed = false;
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines = text.lines().count();
        let key = path.to_string_lossy().replace('\\', "/");
        let allowed = SIZE_DEBT
            .iter()
            .find(|(debt_path, _)| *debt_path == key)
            .map_or(FILE_LINE_CAP, |(_, cap)| *cap);
        if lines > allowed {
            failed = true;
            eprintln!(
                "lint: {key} has {lines} lines (allowed {allowed}).\n  \
                 fix: split it by responsibility into sibling modules (AGENTS.md rule 22). \
                 Do not raise the number in xtask SIZE_DEBT."
            );
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                collect_rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
