use colored::{Color, ColoredString, Colorize};
use ratatui::style::Color as TuiColor;
use std::collections::HashMap;

pub const TAG_COLORS_LIST: &[Color] = &[
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

pub const TUI_TAG_COLORS: &[TuiColor] = &[
    TuiColor::Red,
    TuiColor::Green,
    TuiColor::Yellow,
    TuiColor::Blue,
    TuiColor::Magenta,
    TuiColor::Cyan,
    TuiColor::LightRed,
    TuiColor::LightGreen,
    TuiColor::LightYellow,
    TuiColor::LightBlue,
    TuiColor::LightMagenta,
    TuiColor::LightCyan,
];

pub struct ColorManager {
    tag_colors: HashMap<String, usize>,
}

impl ColorManager {
    pub fn new() -> Self {
        ColorManager {
            tag_colors: HashMap::new(),
        }
    }

    fn get_tag_color_index(&mut self, tag: &str) -> usize {
        let len = self.tag_colors.len();
        *self.tag_colors.entry(tag.to_string()).or_insert(len)
    }

    pub fn get_tag_cli_color(&mut self, tag: &str) -> Color {
        let idx = self.get_tag_color_index(tag);
        TAG_COLORS_LIST[idx % TAG_COLORS_LIST.len()]
    }

    pub fn get_tag_tui_color(&mut self, tag: &str) -> TuiColor {
        let idx = self.get_tag_color_index(tag);
        TUI_TAG_COLORS[idx % TUI_TAG_COLORS.len()]
    }
}

impl Default for ColorManager {
    fn default() -> Self {
        Self::new()
    }
}

use crate::logcat::parser::LogLevel;

pub fn get_level_cli_color(level: &str) -> (ColoredString, Color) {
    match level {
        "D" => (" D ".black().bold().on_bright_blue(), Color::BrightBlue),
        "I" => (" I ".black().bold().on_bright_green(), Color::BrightGreen),
        "W" => (" W ".black().bold().on_yellow(), Color::Yellow),
        "E" => (" E ".black().bold().on_bright_red(), Color::BrightRed),
        "V" => (" V ".black().bold().on_blue(), Color::Blue),
        "F" => (" F ".black().bold().on_red(), Color::Red),
        _ => ("    ".normal(), Color::White),
    }
}

pub fn get_level_tui_color(level: &LogLevel) -> TuiColor {
    match level {
        LogLevel::Verbose => TuiColor::Blue,
        LogLevel::Debug => TuiColor::LightBlue,
        LogLevel::Info => TuiColor::LightGreen,
        LogLevel::Warn => TuiColor::Yellow,
        LogLevel::Error => TuiColor::LightRed,
        LogLevel::Fatal => TuiColor::Red,
        LogLevel::Unknown => TuiColor::White,
    }
}

pub fn get_level_tui_bg(level: &LogLevel) -> TuiColor {
    match level {
        LogLevel::Verbose => TuiColor::Blue,
        LogLevel::Debug => TuiColor::LightBlue,
        LogLevel::Info => TuiColor::LightGreen,
        LogLevel::Warn => TuiColor::Yellow,
        LogLevel::Error => TuiColor::LightRed,
        LogLevel::Fatal => TuiColor::Red,
        LogLevel::Unknown => TuiColor::DarkGray,
    }
}
