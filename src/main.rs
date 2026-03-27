mod cli;
mod color;
mod config;
mod logcat;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::io::IsTerminal;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package name pattern to filter (e.g., com.example.app or com.example.*)
    package_pattern: Option<String>,

    /// Use classic streaming mode instead of TUI
    #[arg(long, default_value_t = false)]
    classic: bool,

    /// Disable timestamp display in the output (classic mode only)
    #[arg(short = 't', long, default_value_t = false)]
    no_timestamp: bool,

    /// Filter by log level (e.g., D,I,W,E,V,F)
    #[arg(short = 'l', long)]
    level: Option<String>,

    /// Filter logs containing this text (case-insensitive)
    #[arg(short = 'f', long)]
    filter: Option<String>,

    /// Exclude logs containing this text (case-insensitive)
    #[arg(short = 'e', long)]
    exclude: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let use_classic = args.classic
        || !std::io::stdout().is_terminal()
        || args.package_pattern.is_some()
        || args.level.is_some()
        || args.filter.is_some()
        || args.exclude.is_some();

    if use_classic {
        cli::run_classic(cli::ClassicArgs {
            package_pattern: args.package_pattern,
            no_timestamp: args.no_timestamp,
            level: args.level,
            filter: args.filter,
            exclude: args.exclude,
        })
    } else {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(tui::run_tui())
    }
}
