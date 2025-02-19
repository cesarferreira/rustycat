use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::process;
use std::cell::RefCell;
use std::collections::HashMap;
use termion::event::Key;
use termion::input::TermRead;
use termion::terminal_size;

const TAG_WIDTH: usize = 25;
const LEFT_PADDING: usize = 2;
const TIMESTAMP_WIDTH: usize = 12;  // Changed to fit "HH:MM:SS.mmm"
const TOTAL_PREFIX_WIDTH: usize = LEFT_PADDING + TIMESTAMP_WIDTH + TAG_WIDTH + 3; // +3 for level and spaces

thread_local! {
    static LAST_TAG: RefCell<String> = RefCell::new(String::new());
    static TAG_COLORS: RefCell<HashMap<String, Color>> = RefCell::new(HashMap::new());
}

const TAG_COLORS_LIST: &[Color] = &[
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::BrightRed,
    Color::BrightGreen,
    Color::BrightYellow,
    Color::BrightBlue,
    Color::BrightMagenta,
    Color::BrightCyan,
];

fn get_tag_color(tag: &str) -> Color {
    TAG_COLORS.with(|colors| {
        let mut colors = colors.borrow_mut();
        if let Some(&color) = colors.get(tag) {
            color
        } else {
            let color = TAG_COLORS_LIST[colors.len() % TAG_COLORS_LIST.len()];
            colors.insert(tag.to_string(), color);
            color
        }
    })
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package name pattern to filter (e.g., com.example.app or com.example.*)
    package_pattern: Option<String>,

    /// Disable timestamp display in the output
    #[arg(short = 't', long, default_value_t = false)]
    no_timestamp: bool,
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

fn extract_log_parts(line: &str) -> Option<(String, String, String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    // Standard logcat format:
    // Date Time PID TID Level Tag: Message
    // 02-03 15:44:41.704 2359 3654 I Tag: Message
    
    // Extract time with milliseconds (15:44:41.704)
    let time_parts: Vec<&str> = parts[1].split('.').collect();
    let time = time_parts[0];
    let ms = time_parts.get(1).unwrap_or(&"000");
    let timestamp = format!("{}.{}", time, &ms[..3]); // Ensure we only take 3 digits for milliseconds
    
    let level = parts[4];
    let tag_and_message = parts[5..].join(" ");
    let (tag, message) = if let Some(pos) = tag_and_message.find(": ") {
        tag_and_message.split_at(pos)
    } else {
        (tag_and_message.as_str(), "")
    };

    Some((
        timestamp,
        tag.trim().to_string(),
        level.to_string(),
        message.trim_start_matches(": ").to_string()
    ))
}

fn get_level_color(level: &str) -> (ColoredString, Color) {
    match level {
        // Debug: Light blue background with white text
        "D" => (" D ".black().bold().on_bright_blue(), Color::BrightBlue),
        // Info: Green background with white text
        "I" => (" I ".black().bold().on_bright_green(), Color::BrightGreen),
        // Warning: Yellow/Orange background with black text
        "W" => (" W ".black().bold().on_yellow(), Color::Yellow),
        // Error: Red background with white text
        "E" => (" E ".black().bold().on_bright_red(), Color::BrightRed),
        // Verbose: Blue background with white text
        "V" => (" V ".black().bold().on_blue(), Color::Blue),
        // Fatal: Bright red background with white text
        "F" => (" F ".black().bold().on_red(), Color::Red),
        _ => ("    ".normal(), Color::White),
    }
}

fn format_multiline_content(content: &str, color: Color, hide_timestamp: bool) -> String {
    // Calculate the message start padding (where the content should align)
    let timestamp_width = if hide_timestamp { 0 } else { TIMESTAMP_WIDTH };
    let message_start_padding = LEFT_PADDING + timestamp_width + TAG_WIDTH + 4 + 2; // +4 for level, +2 for spaces
    let padding = " ".repeat(message_start_padding);
    
    // Get terminal width
    let term_width = terminal_size().map(|(w, _)| w as usize).unwrap_or(80);
    
    let mut result = String::new();
    let mut is_first_line = true;

    for line in content.lines() {
        if !is_first_line {
            result.push_str(&format!("\n{}", padding));
        }
        
        // Available width for the message content
        let available_width = term_width.saturating_sub(message_start_padding);
        let mut remaining = line;
        
        while !remaining.is_empty() {
            let (chunk, rest) = if remaining.len() > available_width {
                // Try to break at the last space within the available width
                let slice = &remaining[..available_width];
                if let Some(last_space) = slice.rfind(' ') {
                    remaining.split_at(last_space)
                } else {
                    // If no space found, break at available width
                    remaining.split_at(available_width)
                }
            } else {
                (remaining, "")
            };
            
            if !is_first_line || !result.is_empty() {
                result.push_str(&format!("\n{}", padding));
            }
            result.push_str(&chunk.color(color).to_string());
            remaining = rest.trim_start();
        }
        
        is_first_line = false;
    }
    
    result
}

fn format_log_line(line: &str, hide_timestamp: bool) -> Option<String> {
    if let Some((timestamp, tag, level, content)) = extract_log_parts(line) {
        let (level_str, color) = get_level_color(&level);
        let padding = " ".repeat(LEFT_PADDING);
        let formatted_content = format_multiline_content(&content, color, hide_timestamp);
        
        // Check if tag has changed
        let show_tag = LAST_TAG.with(|last_tag| {
            let mut last = last_tag.borrow_mut();
            let changed = *last != tag;
            *last = tag.clone();
            changed
        });

        let tag_color = get_tag_color(&tag);
        let tag_display = if show_tag {
            format!("{:>width$}", tag.color(tag_color), width = TAG_WIDTH)
        } else {
            format!("{:>width$}", " ".repeat(tag.len()).color(tag_color), width = TAG_WIDTH)
        };
        
        let timestamp_part = if hide_timestamp {
            "".to_string()
        } else {
            format!("{:<width$} ", timestamp.bright_black(), width = TIMESTAMP_WIDTH)
        };
        
        Some(format!("{}{}{} {} {}", 
            padding,
            timestamp_part,
            tag_display,
            level_str,
            formatted_content
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
            if let Some(formatted) = format_log_line(&line, args.no_timestamp) {
                println!("{}", formatted);
            }
        }
    }

    Ok(())
}
