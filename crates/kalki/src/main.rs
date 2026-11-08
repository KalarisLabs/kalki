//! `kalki` — the Kalki Code CLI. v0.1: doctor + version; sessions in M1.

#![forbid(unsafe_code)]

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
    /// Run environment diagnostics (git, disk, keyring, sandbox, WAL integrity).
    Doctor,
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
            println!("sessions: {}", session_dir().display());
            println!("status: pre-release scaffold — M1 (WAL + session loop) pending");
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
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(std::path::PathBuf::from))
        .or_else(|_| std::env::var("USERPROFILE").map(std::path::PathBuf::from))
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}
