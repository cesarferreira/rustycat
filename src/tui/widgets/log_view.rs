use crate::color::{get_level_tui_bg, get_level_tui_color, ColorManager};
use crate::logcat::parser::LogEntry;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct LogView<'a> {
    pub entries: Vec<&'a LogEntry>,
    pub scroll: usize,
    pub focused: bool,
    pub auto_scroll: bool,
    pub adb_connected: bool,
    pub color_manager: &'a mut ColorManager,
}

impl<'a> LogView<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focused {
                Color::LightCyan
            } else {
                Color::DarkGray
            }))
            .title(" Logs ");

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let visible_height = inner.height as usize;
        let total = self.entries.len();

        if total == 0 {
            let lines = if !self.adb_connected {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ADB disconnected",
                        Style::default()
                            .fg(Color::LightRed)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  No Android device found. Please:",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "    1. Connect a device via USB or start an emulator",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(Span::styled(
                        "    2. Run 'adb devices' to verify connection",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(Span::styled(
                        "    3. Restart rcat",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press q to quit",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Waiting for logs...",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Make sure an Android device is connected via adb.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            };
            let msg = Paragraph::new(lines);
            msg.render(inner, buf);
            return;
        }

        let scroll = if self.auto_scroll {
            total.saturating_sub(visible_height)
        } else {
            self.scroll.min(total.saturating_sub(visible_height))
        };

        let start = scroll;
        let end = (start + visible_height).min(total);

        let color_manager = self.color_manager;
        let lines: Vec<Line> = self.entries[start..end]
            .iter()
            .map(|entry| {
                let tag_color = color_manager.get_tag_tui_color(&entry.tag);
                let level_fg = get_level_tui_color(&entry.level);
                let level_bg = get_level_tui_bg(&entry.level);

                let timestamp = Span::styled(
                    format!("{:<12} ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                );

                let tag = Span::styled(
                    format!("{:>20} ", entry.tag),
                    Style::default().fg(tag_color),
                );

                let level = Span::styled(
                    format!(" {} ", entry.level.as_str()),
                    Style::default()
                        .fg(Color::Black)
                        .bg(level_bg)
                        .add_modifier(Modifier::BOLD),
                );

                let message = Span::styled(
                    format!(" {}", entry.message),
                    Style::default().fg(level_fg),
                );

                Line::from(vec![timestamp, tag, level, message])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
