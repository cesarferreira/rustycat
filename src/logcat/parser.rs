use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    pub fn from_char(c: &str) -> Self {
        match c {
            "V" => LogLevel::Verbose,
            "D" => LogLevel::Debug,
            "I" => LogLevel::Info,
            "W" => LogLevel::Warn,
            "E" => LogLevel::Error,
            "F" => LogLevel::Fatal,
            _ => LogLevel::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "V",
            LogLevel::Debug => "D",
            LogLevel::Info => "I",
            LogLevel::Warn => "W",
            LogLevel::Error => "E",
            LogLevel::Fatal => "F",
            LogLevel::Unknown => "?",
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, LogLevel::Error | LogLevel::Fatal)
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LogEntry {
    pub timestamp: String,
    pub pid: String,
    pub tid: String,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
    pub package: Option<String>,
    pub raw_line: String,
}

impl LogEntry {
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            return None;
        }

        // Standard logcat format:
        // Date Time PID TID Level Tag: Message
        // 02-03 15:44:41.704 2359 3654 I Tag: Message
        let time_parts: Vec<&str> = parts[1].split('.').collect();
        let time = time_parts[0];
        let ms = time_parts.get(1).unwrap_or(&"000");
        let timestamp = format!("{}.{}", time, &ms[..std::cmp::min(3, ms.len())]);

        let pid = parts[2].to_string();
        let tid = parts[3].to_string();
        let level = LogLevel::from_char(parts[4]);

        let tag_and_message = parts[5..].join(" ");
        let (tag, message) = if let Some(pos) = tag_and_message.find(": ") {
            let (t, m) = tag_and_message.split_at(pos);
            (t.trim().to_string(), m.trim_start_matches(": ").to_string())
        } else {
            (tag_and_message.clone(), String::new())
        };

        Some(LogEntry {
            timestamp,
            pid,
            tid,
            level,
            tag,
            message,
            package: None,
            raw_line: line.to_string(),
        })
    }
}
