use crate::color::get_level_tui_bg;
use crate::logcat::parser::LogEntry;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct ErrorPane<'a> {
    pub entries: Vec<&'a LogEntry>,
    pub selected: usize,
    pub offset: usize,
    pub focused: bool,
}

impl<'a> ErrorPane<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        let title = format!(" Errors ({}) ", self.entries.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focused {
                Color::LightCyan
            } else {
                Color::DarkGray
            }))
            .title(title);

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let visible_height = inner.height as usize;
        let total = self.entries.len();

        if total == 0 {
            let msg = Paragraph::new("  No errors yet");
            msg.render(inner, buf);
            return;
        }

        let offset = if self.selected >= self.offset + visible_height {
            self.selected.saturating_sub(visible_height - 1)
        } else if self.selected < self.offset {
            self.selected
        } else {
            self.offset
        };

        let end = (offset + visible_height).min(total);

        let lines: Vec<Line> = self.entries[offset..end]
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let real_idx = offset + i;
                let level_bg = get_level_tui_bg(&entry.level);

                let is_selected = real_idx == self.selected && self.focused;

                let timestamp = Span::styled(
                    format!(" {:<12}", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                );

                let pkg = entry
                    .package
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_string();
                let pkg_span = Span::styled(
                    format!(" {:>20}", pkg),
                    Style::default().fg(Color::Gray),
                );

                let tag = Span::styled(
                    format!(" {:>15}", entry.tag),
                    Style::default().fg(Color::DarkGray),
                );

                let level = Span::styled(
                    format!(" {} ", entry.level.as_str()),
                    Style::default()
                        .fg(Color::Black)
                        .bg(level_bg)
                        .add_modifier(Modifier::BOLD),
                );

                let msg_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::LightRed)
                };

                let message = Span::styled(format!(" {}", entry.message), msg_style);

                Line::from(vec![timestamp, pkg_span, tag, level, message])
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
