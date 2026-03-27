use crate::tui::app::{App, InputMode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub struct StatusBar<'a> {
    pub app: &'a App,
}

impl<'a> StatusBar<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        let key_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::Gray);
        let filter_style = Style::default().fg(Color::Yellow);

        let mut spans = vec![];

        // Show disconnection warning
        if !self.app.adb_connected {
            spans.push(Span::styled(
                " ADB DISCONNECTED ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(" q:Quit ", desc_style));
            let line = Line::from(spans);
            let paragraph = Paragraph::new(line);
            paragraph.render(area, buf);
            return;
        }

        match self.app.input_mode {
            InputMode::Search => {
                spans.push(Span::styled(" ESC ", key_style));
                spans.push(Span::styled(" Cancel  ", desc_style));
                spans.push(Span::styled(" ENTER ", key_style));
                spans.push(Span::styled(" Apply  ", desc_style));
                spans.push(Span::styled(
                    format!(" Search: {}▌", self.app.search_input),
                    Style::default().fg(Color::White),
                ));
            }
            InputMode::Normal => {
                spans.push(Span::styled(" Tab ", key_style));
                spans.push(Span::styled(" Pane  ", desc_style));
                spans.push(Span::styled(" / ", key_style));
                spans.push(Span::styled(" Search  ", desc_style));
                spans.push(Span::styled(" e ", key_style));
                spans.push(Span::styled(" Errors  ", desc_style));
                spans.push(Span::styled(" p ", key_style));
                spans.push(Span::styled(" Apps  ", desc_style));
                spans.push(Span::styled(" c ", key_style));
                spans.push(Span::styled(" Clear  ", desc_style));
                spans.push(Span::styled(" ? ", key_style));
                spans.push(Span::styled(" Help  ", desc_style));
                spans.push(Span::styled(" q ", key_style));
                spans.push(Span::styled(" Quit  ", desc_style));

                // Show filter state
                let selected = self.app.selected_packages();
                if !selected.is_empty() {
                    spans.push(Span::styled(
                        format!(" Filter: {} app(s)", selected.len()),
                        filter_style,
                    ));
                }

                if let Some(level) = &self.app.filter.level {
                    spans.push(Span::styled(
                        format!(" Level: {}+", level),
                        filter_style,
                    ));
                }

                if let Some(search) = &self.app.filter.search_text {
                    spans.push(Span::styled(
                        format!(" Search: \"{}\"", search),
                        filter_style,
                    ));
                }

                // Log count
                spans.push(Span::styled(
                    format!(
                        "  {}/{}",
                        self.app.filtered_indices.len(),
                        self.app.all_logs.len()
                    ),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}
