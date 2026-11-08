//! `cargo xtask` — repo automation. std-only; no external crates by design.
//!
//! Usage: `cargo xtask ci` (full local gate) | `cargo xtask crash` (kill -9 drills)

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("ci") => run_sequence(&[
            ("cargo", &["fmt", "--all", "--", "--check"]),
            (
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            ("cargo", &["test", "--workspace"]),
            ("cargo", &["doc", "--workspace", "--no-deps"]),
        ]),
        Some("crash") => {
            eprintln!(
                "crash drills land in M1 (tests/crash/) — see docs/ARCHITECTURE.md §Durability"
            );
            ExitCode::from(2)
        }
        _ => {
            eprintln!("usage: cargo xtask ci|crash");
            ExitCode::from(2)
        }
    }
}

fn run_sequence(steps: &[(&str, &[&str])]) -> ExitCode {
    for (program, args) in steps {
        eprintln!("==> {program} {}", args.join(" "));
        let status = Command::new(program)
            .args(*args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("failed to spawn {program}: {e}");
                std::process::exit(127);
            });
        if !status.success() {
            eprintln!("step failed: {program} {args:?}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
