use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package name pattern to filter (e.g., com.example.app or com.example.*)
    package_pattern: Option<String>,
}

fn get_pids_for_package(pattern: &str) -> Result<Vec<String>> {
    let regex_pattern = pattern.replace(".", "\\.").replace("*", ".*");
    let regex = Regex::new(&regex_pattern)?;

    let output = Command::new("adb")
        .args(["shell", "ps", "-A"])
        .output()
        .context("Failed to execute adb shell ps command")?;

    let processes = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();

    for line in processes.lines() {
        if regex.is_match(line) {
            if let Some(pid) = line.split_whitespace().nth(1) {
                pids.push(pid.to_string());
            }
        }
    }

    Ok(pids)
}

fn colorize_log_line(line: &str) -> ColoredString {
    if line.contains(" E ") {
        line.red()
    } else if line.contains(" W ") {
        line.yellow()
    } else if line.contains(" I ") {
        line.green()
    } else if line.contains(" D ") {
        line.bright_black()
    } else {
        line.normal()
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut logcat_cmd = Command::new("adb");
    logcat_cmd.arg("logcat");

    // Clear the logcat buffer first
    Command::new("adb")
        .args(["logcat", "-c"])
        .output()
        .context("Failed to clear logcat buffer")?;

    if let Some(package_pattern) = args.package_pattern {
        let pids = get_pids_for_package(&package_pattern)?;
        
        if pids.is_empty() {
            println!("No matching processes found for pattern: {}", package_pattern);
            return Ok(());
        }

        // Add --pid argument for each found PID
        for pid in pids {
            logcat_cmd.arg("--pid").arg(pid);
        }
    }

    let process = logcat_cmd
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start logcat process")?;

    let reader = BufReader::new(process.stdout.unwrap());

    for line in reader.lines() {
        if let Ok(line) = line {
            println!("{}", colorize_log_line(&line));
        }
    }

    Ok(())
}
