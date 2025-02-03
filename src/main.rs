use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::process;
use std::cell::RefCell;
use termion::event::Key;
use termion::input::TermRead;

const TAG_WIDTH: usize = 25;
const LEFT_PADDING: usize = 15;
const TOTAL_PREFIX_WIDTH: usize = LEFT_PADDING + TAG_WIDTH + 3; // +3 for level and spaces

thread_local! {
    static LAST_TAG: RefCell<String> = RefCell::new(String::new());
}

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
        "D" => ("D".bold().on_bright_black(), Color::BrightBlack),
        "I" => ("I".bold().on_green(), Color::Green),
        "W" => ("W".bold().on_yellow(), Color::Yellow),
        "E" => ("E".bold().on_red(), Color::Red),
        "V" => ("V".bold().on_blue(), Color::Blue),
        "F" => ("F".bold().on_red(), Color::BrightRed),
        _ => (" ".normal(), Color::White),
    }
}

fn format_multiline_content(content: &str, color: Color) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 1 {
        return content.color(color).to_string();
    }

    let padding = " ".repeat(TOTAL_PREFIX_WIDTH);
    let mut result = lines[0].color(color).to_string();
    for line in &lines[1..] {
        result.push_str(&format!("\n{}{}", padding, line.color(color)));
    }
    result
}

fn format_log_line(line: &str) -> Option<String> {
    if let Some((tag, level, content)) = extract_log_parts(line) {
        let (level_str, color) = get_level_color(&level);
        let padding = " ".repeat(LEFT_PADDING);
        let formatted_content = format_multiline_content(&content, color);
        
        // Check if tag has changed
        let show_tag = LAST_TAG.with(|last_tag| {
            let mut last = last_tag.borrow_mut();
            let changed = *last != tag;
            *last = tag.clone();
            changed
        });

        let tag_display = if show_tag {
            tag.bright_black()
        } else {
            " ".repeat(tag.len()).bright_black()
        };
        
        Some(format!("{}{:>width$} {} {}", 
            padding,
            tag_display,
            format!(" {} ", level_str), // Add spacing around the level
            formatted_content,
            width = TAG_WIDTH
        ))
    } else {
        Some(line.to_string())
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
            if let Some(formatted) = format_log_line(&line) {
                println!("{}", formatted);
            }
        }
    }

    Ok(())
}
