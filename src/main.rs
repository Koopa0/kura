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
        /// Include System/ (excluded by default).
        #[arg(long)]
        all: bool,
        /// Fail (exit 1) if a finding with this rule id exists; repeatable.
        #[arg(long)]
        deny: Vec<String>,
        /// A prior run's JSONL; report and gate only on findings this run newly introduced.
        #[arg(long)]
        baseline: Option<PathBuf>,
    },
    /// Report MOC coverage + domain list + symbol table + gaps + orphans.
    Coverage,
    /// Ask whether a note for a name already exists (filename / title / alias / title_en).
    /// Exit 0 if it exists, 1 if not — for a dedup check before writing.
    Exists {
        /// The concept or note name to look up.
        name: String,
    },
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Format {
    Json,
    Human,
    /// A fileable markdown report body (printed to stdout; kura never writes files).
    Md,
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
    let Cli {
        command,
        root,
        format,
    } = Cli::parse();
    let root = match root {
        Some(root) => root,
        None => std::env::current_dir().context("get cwd")?,
    };

    match command {
        Command::Check {
            paths,
            all,
            deny,
            baseline,
        } => {
            for rule in &deny {
                anyhow::ensure!(
                    kura::RULE_IDS.contains(&rule.as_str()),
                    "unknown --deny rule {rule:?}; known rules: {}",
                    kura::RULE_IDS.join(", ")
                );
            }
            let mut report = kura::check(&root, &paths, all).context("check")?;
            if let Some(path) = &baseline {
                let jsonl = std::fs::read_to_string(path)
                    .with_context(|| format!("read baseline {}", path.display()))?;
                report.retain_new(&kura::parse_baseline(&jsonl));
            }
            match output_format(format) {
                // json: pure JSONL on stdout. human/md: the packed report on stdout.
                Format::Json => print!("{}", report.to_jsonl().context("serialize findings")?),
                Format::Human => print!("{}", kura::report::human(&report)),
                Format::Md => print!("{}", kura::report::markdown(&report)),
            }
            Ok(if report.gated(&deny) {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Coverage => {
            let graph = kura::load_graph(&root).context("load graph")?;
            let coverage = kura::coverage::compute(&graph);
            let out = match output_format(format) {
                Format::Json => serde_json::to_string(&coverage).context("serialize")? + "\n",
                // md is a check-report format; coverage/exists fall back to the human view.
                Format::Human | Format::Md => render_coverage(&coverage),
            };
            print!("{out}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Exists { name } => {
            let graph = kura::load_graph(&root).context("load graph")?;
            let report = kura::exists::lookup(&graph, &name);
            let out = match output_format(format) {
                Format::Json => serde_json::to_string(&report).context("serialize")? + "\n",
                Format::Human | Format::Md => render_exists(&report),
            };
            print!("{out}");
            Ok(if report.found() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

/// Human rendering of the coverage report.
fn render_coverage(c: &kura::coverage::Coverage) -> String {
    use std::fmt::Write as _;
    let mut s = format!(
        "{} concepts across {} domains\n",
        c.total_concepts,
        c.domains.len()
    );
    for d in &c.domains {
        let _ = writeln!(
            s,
            "  {:<16} {} concepts: {} mounted, {} pending-mount, {} orphan",
            d.domain, d.concepts, d.mounted, d.pending_mount, d.orphan
        );
    }
    if !c.orphans.is_empty() {
        let _ = writeln!(s, "orphans ({}):", c.orphans.len());
        for p in &c.orphans {
            let _ = writeln!(s, "  {p}");
        }
    }
    s
}

/// Human rendering of an existence query.
fn render_exists(report: &kura::exists::Report) -> String {
    use std::fmt::Write as _;
    if report.matches.is_empty() {
        return format!("\"{}\" does not exist\n", report.query);
    }
    let mut s = format!(
        "\"{}\" exists in {} note(s):\n",
        report.query,
        report.matches.len()
    );
    for m in &report.matches {
        let _ = writeln!(s, "  {} (matched {})", m.path, m.field);
    }
    s
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
