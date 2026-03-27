use crate::tui::app::AppInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

pub struct AppPopup<'a> {
    pub apps: &'a [AppInfo],
    pub selected: usize,
    pub offset: usize,
}

impl<'a> AppPopup<'a> {
    pub fn render(self, frame_area: Rect, buf: &mut Buffer) {
        let total = self.apps.len();

        // Dynamic sizing: width=50% (min 40, max 60), height=apps+4 (max 70%)
        let width = (frame_area.width / 2)
            .max(40)
            .min(60)
            .min(frame_area.width.saturating_sub(4));
        let content_height = (total as u16 + 4)
            .max(6)
            .min(frame_area.height * 7 / 10)
            .min(frame_area.height.saturating_sub(4));

        let x = frame_area.width.saturating_sub(width) / 2;
        let y = frame_area.height.saturating_sub(content_height) / 2;
        let popup_area = Rect::new(x, y, width, content_height);

        Clear.render(popup_area, buf);

        let title = format!(" Apps ({}) ", total);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title);

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Reserve 2 lines for footer (blank line + hint)
        let list_height = inner.height.saturating_sub(2) as usize;
        let max_width = inner.width as usize;

        if total == 0 {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No apps discovered yet...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            let list_area =
                Rect::new(inner.x, inner.y, inner.width, list_height.min(inner.height as usize) as u16);
            msg.render(list_area, buf);
        } else {
            let offset = if self.selected >= self.offset + list_height {
                self.selected.saturating_sub(list_height.saturating_sub(1))
            } else if self.selected < self.offset {
                self.selected
            } else {
                self.offset
            };

            let end = (offset + list_height).min(total);

            let lines: Vec<Line> = self.apps[offset..end]
                .iter()
                .enumerate()
                .map(|(i, app)| {
                    let real_idx = offset + i;
                    let check = if app.selected { "[*]" } else { "[ ]" };
                    let fav = if app.favorite { " ★" } else { "" };
                    let label = format!(
                        " {} {}{}  ({})",
                        check, app.package, fav, app.log_count
                    );
                    let truncated = if label.len() > max_width {
                        format!("{}…", &label[..max_width.saturating_sub(1)])
                    } else {
                        label
                    };

                    let style = if real_idx == self.selected {
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if app.selected {
                        Style::default().fg(Color::LightGreen)
                    } else {
                        Style::default().fg(Color::Gray)
                    };

                    Line::from(Span::styled(truncated, style))
                })
                .collect();

            let list_area = Rect::new(inner.x, inner.y, inner.width, list_height as u16);
            let paragraph = Paragraph::new(lines);
            paragraph.render(list_area, buf);
        }

        // Footer hint line
        let footer_y = inner.y + inner.height.saturating_sub(1);
        let footer_area = Rect::new(inner.x, footer_y, inner.width, 1);
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" Space", Style::default().fg(Color::Yellow)),
            Span::styled(":Toggle  ", Style::default().fg(Color::DarkGray)),
            Span::styled("f", Style::default().fg(Color::Yellow)),
            Span::styled(":Fav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(":Close", Style::default().fg(Color::DarkGray)),
        ]));
        footer.render(footer_area, buf);
    }
}
