use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process;
use termion::event::Key;
use termion::input::TermRead;

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

fn get_process_name(pid: &str) -> Option<String> {
    let output = Command::new("adb")
        .args(["shell", "ps", "-p", pid])
        .output()
        .ok()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    output_str.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().last())
        .map(String::from)
}

fn extract_log_parts(line: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    // Standard logcat format:
    // Date Time PID TID Level Tag: Message
    // 02-03 15:44:41.704 2359 3654 I Tag: Message
    
    let level = parts[4];
    let tag_and_message = parts[5..].join(" ");
    let (tag, message) = if let Some(pos) = tag_and_message.find(": ") {
        tag_and_message.split_at(pos)
    } else {
        (tag_and_message.as_str(), "")
    };

    Some((
        tag.trim().to_string(),
        level.to_string(),
        message.trim_start_matches(": ").to_string()
    ))
}

fn get_level_color(level: &str) -> (ColoredString, Color) {
    match level {
        "D" => ("D".on_bright_black(), Color::BrightBlack),
        "I" => ("I".on_green(), Color::Green),
        "W" => ("W".on_yellow(), Color::Yellow),
        "E" => ("E".on_red(), Color::Red),
        "V" => ("V".on_blue(), Color::Blue),
        "F" => ("F".on_red().bold(), Color::BrightRed),
        _ => (" ".normal(), Color::White),
    }
}

fn format_log_line(line: &str) -> String {
    if let Some((tag, level, content)) = extract_log_parts(line) {
        let (level_str, color) = get_level_color(&level);
        
        format!("{:<30} {} {}", 
            tag.bright_black(),
            level_str,
            content.color(color)
        )
    } else {
        line.to_string()
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set up input handling in a separate thread
    std::thread::spawn(|| {
        let stdin = std::io::stdin();
        for key in stdin.keys() {
            if let Ok(key) = key {
                if matches!(key, Key::Char('q') | Key::Char('Q')) {
                    process::exit(0);
                }
            }
        }
    });

    let mut logcat_cmd = Command::new("adb");
    logcat_cmd.args(["logcat", "-v", "threadtime"]);

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
            println!("{}", format_log_line(&line));
        }
    }

    Ok(())
}
