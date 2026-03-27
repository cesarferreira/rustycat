pub mod app;
mod event;
mod ui;
mod widgets;

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io::stdout;
use tokio::process::Command;

use crate::color::ColorManager;
use crate::config;
use crate::logcat::parser::{LogEntry, LogLevel};
use app::{App, InputMode, Pane};
use event::{Event, EventLoop};

const BATCH_SIZE: usize = 100;

pub async fn run_tui() -> Result<()> {
    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut color_manager = ColorManager::new();

    // Load config
    if let Ok(cfg) = config::load_config() {
        app.favorites = cfg.favorites;
        app.last_selected = cfg.last_selected;
    }

    // Clear logcat buffer
    let _ = Command::new("adb")
        .args(["logcat", "-c"])
        .output()
        .await;

    // Spawn logcat process
    let mut logcat = Command::new("adb")
        .args(["logcat", "-v", "threadtime"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start adb logcat. Is adb installed and a device connected?")?;

    let logcat_stdout = logcat
        .stdout
        .take()
        .context("Failed to capture logcat stdout")?;

    // Get initial PID map
    if let Ok(map) = get_pid_package_map_async().await {
        app.update_pid_map(map);
    }

    let mut event_loop = EventLoop::new(logcat_stdout);
    let mut pid_refresh_counter = 0u32;

    // Main loop
    loop {
        // Draw
        terminal.draw(|frame| {
            ui::draw(frame, &app, &mut color_manager);
        })?;

        // Handle first event (blocking wait)
        if let Some(event) = event_loop.next().await {
            process_event(&mut app, event, &mut pid_refresh_counter).await;
        }

        // Drain pending events in batches for high-volume streams
        let pending = event_loop.drain(BATCH_SIZE);
        for event in pending {
            process_event(&mut app, event, &mut pid_refresh_counter).await;
        }

        if app.quit {
            break;
        }
    }

    // Save config before exit
    let cfg = config::Config {
        favorites: app.favorites.clone(),
        last_selected: app.selected_packages(),
    };
    let _ = config::save_config(&cfg);

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    // Kill logcat
    let _ = logcat.kill().await;

    Ok(())
}

async fn process_event(app: &mut App, event: Event, pid_refresh_counter: &mut u32) {
    match event {
        Event::Key(key) => {
            handle_key(app, key);
        }
        Event::LogLine(line) => {
            if let Some(entry) = LogEntry::parse(&line) {
                app.ingest(entry);
            }
        }
        Event::Tick => {
            *pid_refresh_counter += 1;
            // Refresh PID map every ~5 seconds (5000ms / 60ms tick = ~83 ticks)
            if *pid_refresh_counter >= 83 {
                *pid_refresh_counter = 0;
                if let Ok(map) = get_pid_package_map_async().await {
                    app.update_pid_map(map);
                }
            }
        }
        Event::Resize => {
            // Terminal resize is handled automatically by ratatui on next draw
        }
        Event::AdbDisconnected => {
            app.adb_connected = false;
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }

    match app.input_mode {
        InputMode::Search => handle_search_key(app, key),
        InputMode::Normal => handle_normal_key(app, key),
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.clear_search();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.apply_search();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            app.apply_search();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            app.apply_search();
        }
        _ => {}
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
        }
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            } else if app.filter.search_text.is_some() {
                app.clear_search();
            }
        }
        KeyCode::Tab => app.cycle_pane_forward(),
        KeyCode::BackTab => app.cycle_pane_backward(),
        KeyCode::Char('e') => {
            app.show_error_pane = !app.show_error_pane;
            if !app.show_error_pane && app.active_pane == Pane::ErrorPane {
                app.active_pane = Pane::LogView;
            }
        }
        KeyCode::Char('p') => {
            app.show_app_picker = !app.show_app_picker;
            if !app.show_app_picker && app.active_pane == Pane::AppPicker {
                app.active_pane = Pane::LogView;
            }
        }
        KeyCode::Char('c') => app.clear_logs(),

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => handle_nav_down(app),
        KeyCode::Char('k') | KeyCode::Up => handle_nav_up(app),
        KeyCode::Char('g') | KeyCode::Home => handle_nav_top(app),
        KeyCode::Char('G') | KeyCode::End => handle_nav_bottom(app),

        // App picker actions
        KeyCode::Char(' ') => app.toggle_app_selection(),
        KeyCode::Char('f') => {
            app.toggle_app_favorite();
        }
        KeyCode::Enter => handle_enter(app),

        // Level quick filters
        KeyCode::Char('1') => app.set_level_filter(Some(LogLevel::Verbose)),
        KeyCode::Char('2') => app.set_level_filter(Some(LogLevel::Debug)),
        KeyCode::Char('3') => app.set_level_filter(Some(LogLevel::Info)),
        KeyCode::Char('4') => app.set_level_filter(Some(LogLevel::Warn)),
        KeyCode::Char('5') => app.set_level_filter(Some(LogLevel::Error)),
        KeyCode::Char('0') => app.set_level_filter(None),

        _ => {}
    }
}

fn handle_nav_down(app: &mut App) {
    match app.active_pane {
        Pane::LogView => {
            if !app.filtered_indices.is_empty() {
                app.auto_scroll = false;
                let max = app.filtered_indices.len().saturating_sub(1);
                app.log_scroll = (app.log_scroll + 1).min(max);
            }
        }
        Pane::AppPicker => {
            if !app.active_apps.is_empty() {
                app.app_picker_selected =
                    (app.app_picker_selected + 1).min(app.active_apps.len() - 1);
            }
        }
        Pane::ErrorPane => {
            if !app.error_indices.is_empty() {
                app.error_pane_selected =
                    (app.error_pane_selected + 1).min(app.error_indices.len() - 1);
            }
        }
    }
}

fn handle_nav_up(app: &mut App) {
    match app.active_pane {
        Pane::LogView => {
            app.auto_scroll = false;
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        Pane::AppPicker => {
            app.app_picker_selected = app.app_picker_selected.saturating_sub(1);
        }
        Pane::ErrorPane => {
            app.error_pane_selected = app.error_pane_selected.saturating_sub(1);
        }
    }
}

fn handle_nav_top(app: &mut App) {
    match app.active_pane {
        Pane::LogView => {
            app.auto_scroll = false;
            app.log_scroll = 0;
        }
        Pane::AppPicker => {
            app.app_picker_selected = 0;
        }
        Pane::ErrorPane => {
            app.error_pane_selected = 0;
        }
    }
}

fn handle_nav_bottom(app: &mut App) {
    match app.active_pane {
        Pane::LogView => {
            app.auto_scroll = true;
        }
        Pane::AppPicker => {
            if !app.active_apps.is_empty() {
                app.app_picker_selected = app.active_apps.len() - 1;
            }
        }
        Pane::ErrorPane => {
            if !app.error_indices.is_empty() {
                app.error_pane_selected = app.error_indices.len() - 1;
            }
        }
    }
}

fn handle_enter(app: &mut App) {
    if app.active_pane == Pane::ErrorPane && !app.error_indices.is_empty() {
        let error_idx = app.error_indices[app.error_pane_selected];
        // Find this index in filtered_indices to scroll to it
        if let Some(pos) = app.filtered_indices.iter().position(|&i| i == error_idx) {
            app.log_scroll = pos;
            app.auto_scroll = false;
            app.active_pane = Pane::LogView;
        }
    }
}

async fn get_pid_package_map_async() -> Result<HashMap<String, String>> {
    let output = Command::new("adb")
        .args(["shell", "ps", "-A"])
        .output()
        .await
        .context("Failed to execute adb shell ps command")?;

    let processes = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();

    for line in processes.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            let pid = parts[1].to_string();
            let name = parts[8].to_string();
            map.insert(pid, name);
        }
    }

    Ok(map)
}
