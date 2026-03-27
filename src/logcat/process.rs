use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::process::{Command, Stdio};

pub fn get_pids_for_package(pattern: &str) -> Result<Vec<String>> {
    let regex_pattern = pattern.replace(".", "\\.").replace("*", ".*");
    let regex = Regex::new(&regex_pattern)?;

    let output = Command::new("adb")
        .args(["shell", "ps", "-A"])
        .output()
        .context("Failed to execute adb shell ps command")?;

    let processes = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();

    for line in processes.lines() {
        if regex.is_match(line) {
            if let Some(pid) = line.split_whitespace().nth(1) {
                pids.push(pid.to_string());
            }
        }
    }

    Ok(pids)
}

/// Returns a map of PID -> package name by querying `adb shell ps -A`
#[allow(dead_code)]
pub fn get_pid_package_map() -> Result<HashMap<String, String>> {
    let output = Command::new("adb")
        .args(["shell", "ps", "-A"])
        .output()
        .context("Failed to execute adb shell ps command")?;

    let processes = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();

    for line in processes.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Format: USER PID PPID VSZ RSS WCHAN ADDR S NAME
        if parts.len() >= 9 {
            let pid = parts[1].to_string();
            let name = parts[8].to_string();
            map.insert(pid, name);
        }
    }

    Ok(map)
}

pub fn spawn_logcat() -> Result<std::process::Child> {
    // Clear the logcat buffer first
    let _ = Command::new("adb")
        .args(["logcat", "-c"])
        .output()
        .context("Failed to clear logcat buffer")?;

    Command::new("adb")
        .args(["logcat", "-v", "threadtime"])
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start logcat process")
}

pub fn spawn_logcat_for_pids(pids: &[String]) -> Result<std::process::Child> {
    // Clear the logcat buffer first
    let _ = Command::new("adb")
        .args(["logcat", "-c"])
        .output()
        .context("Failed to clear logcat buffer")?;

    let mut cmd = Command::new("adb");
    cmd.args(["logcat", "-v", "threadtime"]);

    for pid in pids {
        cmd.arg("--pid").arg(pid);
    }

    cmd.stdout(Stdio::piped())
        .spawn()
        .context("Failed to start logcat process")
}
