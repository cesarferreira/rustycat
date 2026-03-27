use crate::tui::app::AppInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct AppPicker<'a> {
    pub apps: &'a [AppInfo],
    pub selected: usize,
    pub offset: usize,
    pub focused: bool,
}

impl<'a> AppPicker<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.focused {
                Color::LightCyan
            } else {
                Color::DarkGray
            }))
            .title(" Apps ");

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let visible_height = inner.height as usize;
        let total = self.apps.len();

        if total == 0 {
            let msg = Paragraph::new("  Waiting for logs...");
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
        let max_width = inner.width as usize;

        let mut lines: Vec<Line> = Vec::new();
        let mut last_was_fav = None;

        for (i, app) in self.apps[offset..end].iter().enumerate() {
            let real_idx = offset + i;

            // Add section headers
            if last_was_fav != Some(app.favorite) {
                if app.favorite {
                    lines.push(Line::from(Span::styled(
                        " -- Favorites --",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                } else if last_was_fav == Some(true) {
                    lines.push(Line::from(Span::styled(
                        " -- Active --",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                last_was_fav = Some(app.favorite);
            }

            let check = if app.selected { "[*]" } else { "[ ]" };
            let fav = if app.favorite { " ★" } else { "" };
            let err_str = if app.error_count > 0 {
                format!(" E:{}", app.error_count)
            } else {
                String::new()
            };

            let label = format!(
                " {} {}{}  ({}{})",
                check, app.package, fav, app.log_count, err_str
            );
            let truncated = if label.len() > max_width {
                format!("{}…", &label[..max_width.saturating_sub(1)])
            } else {
                label
            };

            let style = if real_idx == self.selected && self.focused {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if app.selected {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::Gray)
            };

            lines.push(Line::from(Span::styled(truncated, style)));
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
