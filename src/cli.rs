use anyhow::Result;
use colored::Colorize;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};

use crate::color::{get_level_cli_color, ColorManager};
use crate::logcat::parser::LogEntry;
use crate::logcat::process::{get_pids_for_package, spawn_logcat, spawn_logcat_for_pids};

const TAG_WIDTH: usize = 25;
const LEFT_PADDING: usize = 2;
const TIMESTAMP_WIDTH: usize = 12;

thread_local! {
    static LAST_TAG: RefCell<String> = RefCell::new(String::new());
    static COLOR_MANAGER: RefCell<ColorManager> = RefCell::new(ColorManager::new());
}

pub struct ClassicArgs {
    pub package_pattern: Option<String>,
    pub no_timestamp: bool,
    pub level: Option<String>,
    pub filter: Option<String>,
    pub exclude: Option<String>,
}

fn format_multiline_content(content: &str, color: colored::Color, hide_timestamp: bool) -> String {
    let timestamp_width = if hide_timestamp { 0 } else { TIMESTAMP_WIDTH };
    let message_start_padding = LEFT_PADDING + timestamp_width + TAG_WIDTH + 4 + 2;
    let padding = " ".repeat(message_start_padding);

    let term_width = termion::terminal_size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);

    let mut result = String::new();
    let mut is_first_line = true;

    for line in content.lines() {
        if !is_first_line {
            result.push_str(&format!("\n{}", padding));
        }

        let available_width = term_width.saturating_sub(message_start_padding);
        let mut remaining = line;

        while !remaining.is_empty() {
            let (chunk, rest) = if remaining.len() > available_width {
                let slice = &remaining[..available_width];
                if let Some(last_space) = slice.rfind(' ') {
                    remaining.split_at(last_space)
                } else {
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

fn format_log_line(entry: &LogEntry, hide_timestamp: bool) -> String {
    let (level_str, color) = get_level_cli_color(entry.level.as_str());
    let padding = " ".repeat(LEFT_PADDING);
    let formatted_content = format_multiline_content(&entry.message, color, hide_timestamp);

    let show_tag = LAST_TAG.with(|last_tag| {
        let mut last = last_tag.borrow_mut();
        let changed = *last != entry.tag;
        *last = entry.tag.clone();
        changed
    });

    let tag_color = COLOR_MANAGER.with(|cm| cm.borrow_mut().get_tag_cli_color(&entry.tag));
    let tag_display = if show_tag {
        format!(
            "{:>width$}",
            entry.tag.color(tag_color),
            width = TAG_WIDTH
        )
    } else {
        format!(
            "{:>width$}",
            " ".repeat(entry.tag.len()).color(tag_color),
            width = TAG_WIDTH
        )
    };

    let timestamp_part = if hide_timestamp {
        "".to_string()
    } else {
        format!(
            "{:<width$} ",
            entry.timestamp.bright_black(),
            width = TIMESTAMP_WIDTH
        )
    };

    format!(
        "{}{}{} {} {}",
        padding, timestamp_part, tag_display, level_str, formatted_content
    )
}

fn should_display_log(entry: &LogEntry, args: &ClassicArgs) -> bool {
    if let Some(level_filter) = &args.level {
        if !level_filter
            .split(',')
            .any(|l| l.trim() == entry.level.as_str())
        {
            return false;
        }
    }

    if let Some(filter) = &args.filter {
        if !entry
            .message
            .to_lowercase()
            .contains(&filter.to_lowercase())
        {
            return false;
        }
    }

    if let Some(exclude) = &args.exclude {
        if entry
            .message
            .to_lowercase()
            .contains(&exclude.to_lowercase())
        {
            return false;
        }
    }

    true
}

pub fn run_classic(args: ClassicArgs) -> Result<()> {
    // Set up input handling in a separate thread
    std::thread::spawn(|| {
        use termion::event::Key;
        use termion::input::TermRead;
        let stdin = std::io::stdin();
        for key in stdin.keys() {
            if let Ok(key) = key {
                if matches!(key, Key::Char('q') | Key::Char('Q')) {
                    std::process::exit(0);
                }
            }
        }
    });

    let process = if let Some(package_pattern) = args.package_pattern.as_ref() {
        let pids = get_pids_for_package(package_pattern)?;
        if pids.is_empty() {
            println!(
                "No matching processes found for pattern: {}",
                package_pattern
            );
            return Ok(());
        }
        spawn_logcat_for_pids(&pids)?
    } else {
        spawn_logcat()?
    };

    let reader = BufReader::new(process.stdout.unwrap());

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(entry) = LogEntry::parse(&line) {
                if should_display_log(&entry, &args) {
                    println!("{}", format_log_line(&entry, args.no_timestamp));
                }
            } else {
                println!("{}", line);
            }
        }
    }

    Ok(())
}
