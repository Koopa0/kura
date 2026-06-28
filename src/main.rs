//! kura CLI —— 薄殼。所有邏輯在 lib(`kura::`)。
//!
//! 退出碼(對齊 schema_lint 0/1/2):
//!   0 = 乾淨(無 deny 級 finding)
//!   1 = gate-hit(有 deny 級)
//!   2 = tool-error(壞 root / parse panic 等)—— 由 anyhow 經 main 的 Err 路徑回傳

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "kura", version, about = "蔵 — read-only 知識圖守衛 / 索引器")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// vault 根(預設 cwd);可指 git worktree。
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// 輸出格式;預設依 tty 自動(json 給 agent、human 給人)。
    #[arg(long, global = true, value_enum)]
    format: Option<Format>,
}

#[derive(Subcommand)]
enum Command {
    /// 掃語料層問題(斷鏈/撞名/provenance/map↔disk)。path 只 filter 輸出,圖永遠全掃。
    Check {
        /// 只報這些 path 的 finding(圖仍從整個 root 建)。
        paths: Vec<String>,
        /// 含 System/(預設只掃 knowledge dirs)。
        #[arg(long)]
        all: bool,
        /// 此 rule/severity 以上 → 非零 exit(CI/gate)。
        #[arg(long)]
        deny: Option<String>,
    },
    /// 生成 MOC 覆蓋率 + domain 清單 + symbol table + 缺口 + orphan。
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
            ExitCode::from(2) // tool-error,與 gate-hit(1)區分
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let root = cli
        .root
        .clone()
        .unwrap_or(std::env::current_dir().context("取得 cwd")?);

    match cli.command {
        Command::Check { paths, all, deny } => {
            let _ = all; // TODO(P1): scope 政策(預設排除 System/)
            let mut report = kura::check(&root, &paths).context("check")?;
            report.sort();
            // TODO(P2): 依 --format 出 JSONL(stdout 純)或 human 摘要(blast-radius 排序)
            emit_stub(&report);
            let deny_hit = deny.is_some() && report_has_deny(&report, deny.as_deref());
            Ok(if deny_hit {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Coverage => {
            // TODO(P3): 覆蓋率 + domain + symbol table + 缺口 + orphan
            eprintln!("kura coverage: P3 未實作(scaffold)");
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn emit_stub(report: &kura::Report) {
    eprintln!(
        "kura check: P1/P2 未實作(scaffold)。findings={} error={} warn={} info={}",
        report.findings.len(),
        report.count(kura::Severity::Error),
        report.count(kura::Severity::Warn),
        report.count(kura::Severity::Info),
    );
}

fn report_has_deny(report: &kura::Report, deny: Option<&str>) -> bool {
    match deny {
        Some("error") => report.has_at_least(kura::Severity::Error),
        Some("warn") => report.has_at_least(kura::Severity::Warn),
        _ => false,
    }
}
