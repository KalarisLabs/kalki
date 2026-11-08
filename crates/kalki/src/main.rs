//! `kalki` — the Kalki Code CLI. v0.2: doctor + run (headless one-shot).

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
    /// Run environment diagnostics (git, provider keys, WAL).
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
                "KALKI_PROVIDER",
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
            println!("status: v0.2 — one-shot session loop live (`kalki run -p ...`)");
        }
        Commands::Run {
            objective,
            model,
            max_turns,
        } => {
            let model = model
                .or_else(|| std::env::var("KALKI_MODEL").ok())
                .unwrap_or_else(|| "gpt-5.6".to_string());
            let workspace = std::env::current_dir()?;
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let outcome = rt.block_on(kalki_core::session::run_from_env(
                objective, model, max_turns, workspace,
            ))?;
            println!(
                "\n=== session {} ({} turns{}): ===\n{}",
                outcome.session_id,
                outcome.turns,
                if outcome.aborted { ", aborted" } else { "" },
                outcome.final_text
            );
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
