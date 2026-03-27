use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::logcat::parser::LogEntry;

const ADB_TIMEOUT: Duration = Duration::from_secs(3);

static INDEX_HTML: &str = include_str!("../static/index.html");

pub async fn run_web(port: u16) -> Result<()> {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(ws_handler));

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    println!("RustyCat Web UI listening on http://localhost:{}", port);

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn ws_handler(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    // Check ADB availability
    let adb_available = check_adb().await;

    let status_msg = serde_json::json!({
        "type": "status",
        "adb_connected": adb_available
    });
    let _ = socket.send(Message::Text(status_msg.to_string().into())).await;

    if !adb_available {
        // Keep connection open, wait for client messages
        while let Some(Ok(_msg)) = socket.recv().await {}
        return;
    }

    // Clear logcat buffer
    let _ = timeout(
        ADB_TIMEOUT,
        Command::new("adb").args(["logcat", "-c"]).output(),
    )
    .await;

    // Spawn logcat process
    let mut logcat = match Command::new("adb")
        .args(["logcat", "-v", "threadtime"])
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            let msg = serde_json::json!({"type": "status", "adb_connected": false});
            let _ = socket.send(Message::Text(msg.to_string().into())).await;
            return;
        }
    };

    let stdout = match logcat.stdout.take() {
        Some(s) => s,
        None => return,
    };

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // Get initial PID map
    let mut pid_map = get_pid_map().await.unwrap_or_default();

    let mut pid_refresh = tokio::time::interval(Duration::from_secs(5));
    pid_refresh.tick().await; // consume first immediate tick

    loop {
        tokio::select! {
            line_result = lines.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        if let Some(mut entry) = LogEntry::parse(&line) {
                            // Enrich with package name
                            if let Some(pkg) = pid_map.get(&entry.pid) {
                                entry.package = Some(pkg.clone());
                            }
                            let json = serde_json::json!({
                                "type": "log",
                                "timestamp": entry.timestamp,
                                "pid": entry.pid,
                                "tid": entry.tid,
                                "level": entry.level,
                                "tag": entry.tag,
                                "message": entry.message,
                                "package": entry.package,
                            });
                            if socket.send(Message::Text(json.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        // logcat stream ended
                        let msg = serde_json::json!({"type": "status", "adb_connected": false});
                        let _ = socket.send(Message::Text(msg.to_string().into())).await;
                        break;
                    }
                    Err(_) => break,
                }
            }
            _ = pid_refresh.tick() => {
                if let Ok(new_map) = get_pid_map().await {
                    pid_map = new_map;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if parsed.get("type").and_then(|v| v.as_str()) == Some("clear") {
                                let _ = Command::new("adb")
                                    .args(["logcat", "-c"])
                                    .output()
                                    .await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    let _ = logcat.kill().await;
}

async fn check_adb() -> bool {
    match timeout(ADB_TIMEOUT, Command::new("adb").args(["devices"]).output()).await {
        Ok(Ok(output)) => {
            let out = String::from_utf8_lossy(&output.stdout);
            out.lines().count() > 1 && out.lines().skip(1).any(|l| !l.trim().is_empty())
        }
        _ => false,
    }
}

async fn get_pid_map() -> Result<HashMap<String, String>> {
    let output = Command::new("adb")
        .args(["shell", "ps", "-A"])
        .output()
        .await
        .context("Failed to execute adb shell ps")?;

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
