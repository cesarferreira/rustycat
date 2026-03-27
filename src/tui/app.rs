use crate::logcat::filter::LogFilter;
use crate::logcat::parser::{LogEntry, LogLevel};
use std::collections::{HashMap, VecDeque};

const MAX_LOG_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pane {
    LogView,
    AppPicker,
    ErrorPane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub package: String,
    pub log_count: usize,
    pub error_count: usize,
    pub selected: bool,
    pub favorite: bool,
}

pub struct App {
    pub all_logs: VecDeque<LogEntry>,
    pub filtered_indices: Vec<usize>,
    pub error_indices: Vec<usize>,
    pub filter: LogFilter,
    pub pid_map: HashMap<String, String>,

    pub active_apps: Vec<AppInfo>,

    // UI state
    pub quit: bool,
    pub active_pane: Pane,
    pub input_mode: InputMode,
    pub search_input: String,

    // Log view scroll
    pub log_scroll: usize,
    pub auto_scroll: bool,

    // App picker scroll
    pub app_picker_offset: usize,
    pub app_picker_selected: usize,

    // Error pane scroll
    pub error_pane_offset: usize,
    pub error_pane_selected: usize,

    // Toggles
    pub show_app_picker: bool,
    pub show_error_pane: bool,
    pub show_help: bool,

    // Favorites
    pub favorites: Vec<String>,
    pub last_selected: Vec<String>,

    // Connection state
    pub adb_connected: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            all_logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            filtered_indices: Vec::new(),
            error_indices: Vec::new(),
            filter: LogFilter::new(),
            pid_map: HashMap::new(),
            active_apps: Vec::new(),
            quit: false,
            active_pane: Pane::LogView,
            input_mode: InputMode::Normal,
            search_input: String::new(),
            log_scroll: 0,
            auto_scroll: true,
            app_picker_offset: 0,
            app_picker_selected: 0,
            error_pane_offset: 0,
            error_pane_selected: 0,
            show_app_picker: true,
            show_error_pane: true,
            show_help: false,
            favorites: Vec::new(),
            last_selected: Vec::new(),
            adb_connected: true,
        }
    }

    pub fn ingest(&mut self, mut entry: LogEntry) {
        // Enrich with package name
        if let Some(pkg) = self.pid_map.get(&entry.pid) {
            entry.package = Some(pkg.clone());
            self.update_app_info(pkg.clone(), &entry);
        }

        let idx = self.all_logs.len();

        // Track errors
        if entry.level.is_error() {
            self.error_indices.push(idx);
        }

        // Check filter
        if self.filter.matches(&entry) {
            self.filtered_indices.push(idx);
        }

        self.all_logs.push_back(entry);

        // Enforce capacity
        if self.all_logs.len() > MAX_LOG_ENTRIES {
            self.all_logs.pop_front();
            // Shift all indices down by 1
            self.filtered_indices.retain_mut(|i| {
                if *i == 0 {
                    false
                } else {
                    *i -= 1;
                    true
                }
            });
            self.error_indices.retain_mut(|i| {
                if *i == 0 {
                    false
                } else {
                    *i -= 1;
                    true
                }
            });
        }
    }

    fn update_app_info(&mut self, package: String, entry: &LogEntry) {
        if let Some(app) = self.active_apps.iter_mut().find(|a| a.package == package) {
            app.log_count += 1;
            if entry.level.is_error() {
                app.error_count += 1;
            }
        } else {
            let is_fav = self.favorites.contains(&package);
            let is_selected = self.last_selected.contains(&package);
            self.active_apps.push(AppInfo {
                package: package.clone(),
                log_count: 1,
                error_count: if entry.level.is_error() { 1 } else { 0 },
                selected: is_selected,
                favorite: is_fav,
            });
            // Keep sorted: favorites first, then alphabetical
            self.sort_apps();
        }
    }

    fn sort_apps(&mut self) {
        self.active_apps.sort_by(|a, b| {
            b.favorite
                .cmp(&a.favorite)
                .then_with(|| a.package.cmp(&b.package))
        });
    }

    pub fn refilter(&mut self) {
        // Update filter with selected packages
        self.filter.selected_packages = self
            .active_apps
            .iter()
            .filter(|a| a.selected)
            .map(|a| a.package.clone())
            .collect();

        self.filtered_indices.clear();
        for (idx, entry) in self.all_logs.iter().enumerate() {
            if self.filter.matches(entry) {
                self.filtered_indices.push(idx);
            }
        }
    }

    pub fn clear_logs(&mut self) {
        self.all_logs.clear();
        self.filtered_indices.clear();
        self.error_indices.clear();
        self.log_scroll = 0;
        self.auto_scroll = true;
        for app in &mut self.active_apps {
            app.log_count = 0;
            app.error_count = 0;
        }
    }

    pub fn update_pid_map(&mut self, new_map: HashMap<String, String>) {
        self.pid_map = new_map;
    }

    pub fn cycle_pane_forward(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::LogView => {
                if self.show_app_picker {
                    Pane::AppPicker
                } else if self.show_error_pane {
                    Pane::ErrorPane
                } else {
                    Pane::LogView
                }
            }
            Pane::AppPicker => {
                if self.show_error_pane {
                    Pane::ErrorPane
                } else {
                    Pane::LogView
                }
            }
            Pane::ErrorPane => Pane::LogView,
        };
    }

    pub fn cycle_pane_backward(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::LogView => {
                if self.show_error_pane {
                    Pane::ErrorPane
                } else if self.show_app_picker {
                    Pane::AppPicker
                } else {
                    Pane::LogView
                }
            }
            Pane::AppPicker => Pane::LogView,
            Pane::ErrorPane => {
                if self.show_app_picker {
                    Pane::AppPicker
                } else {
                    Pane::LogView
                }
            }
        };
    }

    pub fn toggle_app_selection(&mut self) {
        if self.active_pane == Pane::AppPicker && !self.active_apps.is_empty() {
            let idx = self.app_picker_selected;
            if idx < self.active_apps.len() {
                self.active_apps[idx].selected = !self.active_apps[idx].selected;
                self.refilter();
            }
        }
    }

    pub fn toggle_app_favorite(&mut self) {
        if self.active_pane == Pane::AppPicker && !self.active_apps.is_empty() {
            let idx = self.app_picker_selected;
            if idx < self.active_apps.len() {
                self.active_apps[idx].favorite = !self.active_apps[idx].favorite;
                let pkg = self.active_apps[idx].package.clone();
                if self.active_apps[idx].favorite {
                    if !self.favorites.contains(&pkg) {
                        self.favorites.push(pkg);
                    }
                } else {
                    self.favorites.retain(|f| f != &pkg);
                }
                self.sort_apps();
            }
        }
    }

    pub fn selected_packages(&self) -> Vec<String> {
        self.active_apps
            .iter()
            .filter(|a| a.selected)
            .map(|a| a.package.clone())
            .collect()
    }

    pub fn set_level_filter(&mut self, level: Option<LogLevel>) {
        self.filter.level = level;
        self.refilter();
    }

    pub fn apply_search(&mut self) {
        if self.search_input.is_empty() {
            self.filter.search_text = None;
        } else {
            self.filter.search_text = Some(self.search_input.clone());
        }
        self.refilter();
    }

    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.filter.search_text = None;
        self.refilter();
    }
}
