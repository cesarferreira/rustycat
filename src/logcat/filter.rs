use super::parser::{LogEntry, LogLevel};

#[derive(Debug, Clone)]
pub struct LogFilter {
    pub level: Option<LogLevel>,
    pub search_text: Option<String>,
    pub exclude_text: Option<String>,
    pub selected_packages: Vec<String>,
}

impl LogFilter {
    pub fn new() -> Self {
        LogFilter {
            level: None,
            search_text: None,
            exclude_text: None,
            selected_packages: Vec::new(),
        }
    }

    pub fn matches(&self, entry: &LogEntry) -> bool {
        // Check level filter
        if let Some(min_level) = &self.level {
            let level_order = |l: &LogLevel| -> u8 {
                match l {
                    LogLevel::Verbose => 0,
                    LogLevel::Debug => 1,
                    LogLevel::Info => 2,
                    LogLevel::Warn => 3,
                    LogLevel::Error => 4,
                    LogLevel::Fatal => 5,
                    LogLevel::Unknown => 0,
                }
            };
            if level_order(&entry.level) < level_order(min_level) {
                return false;
            }
        }

        // Check search text
        if let Some(search) = &self.search_text {
            let search_lower = search.to_lowercase();
            if !entry.message.to_lowercase().contains(&search_lower)
                && !entry.tag.to_lowercase().contains(&search_lower)
            {
                return false;
            }
        }

        // Check exclude text
        if let Some(exclude) = &self.exclude_text {
            if entry
                .message
                .to_lowercase()
                .contains(&exclude.to_lowercase())
            {
                return false;
            }
        }

        // Check selected packages (empty = show all)
        if !self.selected_packages.is_empty() {
            if let Some(pkg) = &entry.package {
                if !self.selected_packages.contains(pkg) {
                    return false;
                }
            } else {
                // No package info and we have a filter - hide it
                return false;
            }
        }

        true
    }
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new()
    }
}
