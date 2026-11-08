//! `kalki` — the Kalki Code CLI. v0.2: doctor; `run` wires in after the P3/P7 merge.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Kalki Code — a self-recursive continual coding harness.
#[derive(Parser, Debug)]
#[command(name = "kalki", version, about, long_about = None)]
struct Cli {
    /// Emit logs as JSONL (also respects `KALKI_LOG`).
    #[arg(long, global = true)]
    json_logs: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run environment diagnostics (git, disk, provider keys, WAL).
    Doctor,
    /// Run one session against the configured provider and print the outcome.
    Run {
        /// The objective/prompt for this run.
        #[arg(short = 'p', long)]
        objective: String,
        /// Model identifier (defaults to `KALKI_MODEL` or `gpt-5.6`).
        #[arg(long)]
        model: Option<String>,
        /// Hard cap on model turns.
        #[arg(long, default_value_t = 200)]
        max_turns: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = std::env::var("KALKI_LOG").unwrap_or_else(|_| "info".to_string());
    if cli.json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    match cli.command.unwrap_or(Commands::Doctor) {
        Commands::Doctor => {
            println!("kalki doctor — v{}", env!("CARGO_PKG_VERSION"));
            println!("workspace: {}", std::env::current_dir()?.display());
            println!(
                "git: {}",
                if git_present() {
                    "OK"
                } else {
                    "MISSING (required)"
                }
            );
            for var in [
                "OPENAI_API_KEY",
                "ANTHROPIC_API_KEY",
                "OPENAI_COMPAT_API_KEY",
            ] {
                println!(
                    "env {var}: {}",
                    if std::env::var(var).is_ok() {
                        "set"
                    } else {
                        "not set"
                    }
                );
            }
            println!("sessions: {}", session_dir().display());
            println!("status: v0.2 scaffold — session loop landing on main behind green CI");
        }
        Commands::Run {
            objective,
            model,
            max_turns,
        } => {
            let _ = (objective, model, max_turns);
            eprintln!(
                "kalki run: the v0.2 session loop lands on main after the impl branches merge (CHANGELOG.md, ROADMAP.md).\n\
                 Track progress: git log main; final wiring happens in the merge milestone."
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

fn git_present() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn session_dir() -> std::path::PathBuf {
    dirs_home().join(".kalki").join("sessions")
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var("KALKI_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(PathBuf::from))
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|_| PathBuf::from("."))
}
