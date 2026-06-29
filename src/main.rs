//! kura CLI — a thin shell. All logic lives in the library (`kura::`).
//!
//! Exit codes:
//!   0 = clean (no deny-level finding)
//!   1 = gate-hit (a deny-level finding exists)
//!   2 = tool-error (bad root / parse panic) — returned via main's Err path

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "kura",
    version,
    about = "read-only knowledge-graph guardian / indexer"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Vault root (defaults to cwd); may point at a git worktree.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Output format; defaults by tty (json for agents, human for people).
    #[arg(long, global = true, value_enum)]
    format: Option<Format>,
}

#[derive(Subcommand)]
enum Command {
    /// Scan for corpus-level problems (broken links / collisions / provenance / map-vs-disk).
    /// Path args only filter output; the graph is always built from the whole root.
    Check {
        /// Only report findings for these paths (the graph is still built from the whole root).
        paths: Vec<String>,
        /// Include System/ (by default only knowledge dirs are scanned).
        #[arg(long)]
        all: bool,
        /// Exit non-zero when a finding at this rule/severity or above exists (CI/gate).
        #[arg(long)]
        deny: Option<String>,
    },
    /// Report MOC coverage + domain list + symbol table + gaps + orphans.
    Coverage,
}

#[derive(Copy, Clone, ValueEnum)]
enum Format {
    Json,
    Human,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("kura: {e:#}");
            ExitCode::from(2) // tool-error, distinct from gate-hit (1)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let root = cli
        .root
        .clone()
        .unwrap_or(std::env::current_dir().context("get cwd")?);

    match cli.command {
        Command::Check { paths, all, deny } => {
            let _ = all; // scope policy (default excludes System/) not wired yet
            let report = kura::check(&root, &paths).context("check")?;
            match output_format(cli.format) {
                // json: pure JSONL on stdout. human: the summary on stdout.
                Format::Json => print!("{}", report.to_jsonl().context("serialize findings")?),
                Format::Human => print!("{}", report.summary()),
            }
            let deny_hit = report_has_deny(&report, deny.as_deref());
            Ok(if deny_hit {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Coverage => {
            eprintln!("kura coverage: not implemented yet");
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Resolve the output format: explicit flag wins, otherwise json for a pipe, human for a terminal.
fn output_format(flag: Option<Format>) -> Format {
    flag.unwrap_or_else(|| {
        if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
            Format::Human
        } else {
            Format::Json
        }
    })
}

fn report_has_deny(report: &kura::Report, deny: Option<&str>) -> bool {
    match deny {
        Some("error") => report.has_at_least(kura::Severity::Error),
        Some("warn") => report.has_at_least(kura::Severity::Warn),
        _ => false,
    }
}
