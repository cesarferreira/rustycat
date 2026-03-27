use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::color::ColorManager;
use crate::tui::app::{App, Pane};
use crate::tui::widgets::app_picker::AppPicker;
use crate::tui::widgets::error_pane::ErrorPane;
use crate::tui::widgets::log_view::LogView;
use crate::tui::widgets::status_bar::StatusBar;

pub fn draw(frame: &mut Frame, app: &App, color_manager: &mut ColorManager) {
    let size = frame.area();

    // Main vertical layout: [content] [status bar]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    let content_area = main_chunks[0];
    let status_area = main_chunks[1];

    // Content area split: [main area] [error pane (optional)]
    let content_chunks = if app.show_error_pane {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(8)])
            .split(content_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(content_area)
    };

    let main_area = content_chunks[0];
    let error_area = content_chunks[1];

    // Main area split: [app picker (optional)] [log view]
    let log_area = if app.show_app_picker {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(1)])
            .split(main_area);

        // Render app picker
        let picker = AppPicker {
            apps: &app.active_apps,
            selected: app.app_picker_selected,
            offset: app.app_picker_offset,
            focused: app.active_pane == Pane::AppPicker,
        };
        picker.render(h_chunks[0], frame.buffer_mut());

        h_chunks[1]
    } else {
        main_area
    };

    // Render log view
    let entries: Vec<_> = app
        .filtered_indices
        .iter()
        .filter_map(|&idx| app.all_logs.get(idx))
        .collect();

    let log_view = LogView {
        entries,
        scroll: app.log_scroll,
        focused: app.active_pane == Pane::LogView,
        auto_scroll: app.auto_scroll,
        color_manager,
    };
    log_view.render(log_area, frame.buffer_mut());

    // Render error pane
    if app.show_error_pane {
        let error_entries: Vec<_> = app
            .error_indices
            .iter()
            .filter_map(|&idx| app.all_logs.get(idx))
            .collect();

        let error_pane = ErrorPane {
            entries: error_entries,
            selected: app.error_pane_selected,
            offset: app.error_pane_offset,
            focused: app.active_pane == Pane::ErrorPane,
        };
        error_pane.render(error_area, frame.buffer_mut());
    }

    // Render status bar
    let status = StatusBar { app };
    status.render(status_area, frame.buffer_mut());

    // Render help overlay
    if app.show_help {
        render_help_overlay(frame, size);
    }
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 22u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            " Key Bindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Tab/S-Tab ", Style::default().fg(Color::Yellow)),
            Span::raw("  Cycle pane focus"),
        ]),
        Line::from(vec![
            Span::styled(" q/Ctrl+C  ", Style::default().fg(Color::Yellow)),
            Span::raw("  Quit"),
        ]),
        Line::from(vec![
            Span::styled(" /         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Search logs"),
        ]),
        Line::from(vec![
            Span::styled(" Esc       ", Style::default().fg(Color::Yellow)),
            Span::raw("  Clear search / exit"),
        ]),
        Line::from(vec![
            Span::styled(" e         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle error pane"),
        ]),
        Line::from(vec![
            Span::styled(" p         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle app picker"),
        ]),
        Line::from(vec![
            Span::styled(" j/k ↑/↓   ", Style::default().fg(Color::Yellow)),
            Span::raw("  Navigate"),
        ]),
        Line::from(vec![
            Span::styled(" g/G       ", Style::default().fg(Color::Yellow)),
            Span::raw("  Top / Bottom (resume scroll)"),
        ]),
        Line::from(vec![
            Span::styled(" Space     ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle app selection"),
        ]),
        Line::from(vec![
            Span::styled(" f         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle favorite"),
        ]),
        Line::from(vec![
            Span::styled(" Enter     ", Style::default().fg(Color::Yellow)),
            Span::raw("  Jump to error in logs"),
        ]),
        Line::from(vec![
            Span::styled(" c         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Clear logs"),
        ]),
        Line::from(vec![
            Span::styled(" 1-5       ", Style::default().fg(Color::Yellow)),
            Span::raw("  Level filter (V/D/I/W/E)"),
        ]),
        Line::from(vec![
            Span::styled(" 0         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Clear level filter"),
        ]),
        Line::from(vec![
            Span::styled(" ?         ", Style::default().fg(Color::Yellow)),
            Span::raw("  Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}
