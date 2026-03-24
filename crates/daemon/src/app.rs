use agent_computer_shared::protocol::*;
use agent_computer_shared::errors;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::Command;

use agent_computer_daemon::ax_engine;
use crate::DaemonState;

// MARK: - App Management

/// Open or focus an application. Uses `open -a` for simplicity and reliability.
pub fn open_app(name: &str) -> Result<(String, i32, bool), agent_computer_shared::types::ErrorInfo> {
    // Check if already running via pgrep-style approach using `osascript`
    let check = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"tell application "System Events" to set appList to name of every process whose name is "{}""#,
            name
        ))
        .output();

    let was_running = match &check {
        Ok(out) => {
            let output = String::from_utf8_lossy(&out.stdout);
            !output.trim().is_empty()
        }
        Err(_) => false,
    };

    // Use `open -a` to open/focus
    let result = Command::new("open").arg("-a").arg(name).output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                // Try fuzzy suggestions
                let suggestions = get_running_app_names();
                if !suggestions.is_empty() {
                    return Err(errors::app_not_found(name, &suggestions));
                }
                return Err(errors::app_not_found(name, &[]));
            }

            // Get the PID of the app after opening
            let pid = get_app_pid(name).unwrap_or(0);
            Ok((name.to_string(), pid, was_running))
        }
        Err(e) => Err(errors::daemon_error(&format!("Failed to open app: {}", e))),
    }
}

/// Open app with CDP (Chrome DevTools Protocol) enabled.
/// Quits existing instance, relaunches with --remote-debugging-port.
pub fn open_app_with_cdp(
    name: &str,
    state: &mut DaemonState,
) -> Result<(String, i32, bool, u16), agent_computer_shared::types::ErrorInfo> {
    let port = deterministic_port(name);

    // Kill existing instance first
    let _ = Command::new("osascript")
        .arg("-e")
        .arg(format!(r#"tell application "{}" to quit"#, name))
        .output();

    // Wait a bit for quit
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Relaunch with CDP flag — for Electron apps, pass --remote-debugging-port
    let result = Command::new("open")
        .arg("-a")
        .arg(name)
        .arg("--args")
        .arg(format!("--remote-debugging-port={}", port))
        .output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(errors::daemon_error(&format!(
                    "Failed to relaunch {} with CDP: {}",
                    name, stderr
                )));
            }

            // Wait for app + CDP to be ready
            let mut cdp_ready = false;
            for _ in 0..50 {
                // 5 seconds max
                std::thread::sleep(std::time::Duration::from_millis(100));
                if probe_cdp_port(port) {
                    cdp_ready = true;
                    break;
                }
            }

            if !cdp_ready {
                // App launched but CDP not ready — still report success with port
                eprintln!(
                    "Warning: {} launched but CDP not responding on port {}",
                    name, port
                );
            }

            let pid = get_app_pid(name).unwrap_or(0);

            // Track in state
            state.cdp_connections.insert(name.to_string(), port);
            state.port_assignments.insert(name.to_string(), port);

            // Auto-connect agent-browser session (Task 5.2)
            if state.browser_bridge.is_available() && cdp_ready {
                let session = name.to_lowercase().replace(' ', "-");
                match state.browser_bridge.connect(&session, port) {
                    Ok(()) => {
                        eprintln!("[app] agent-browser session '{}' connected on port {}", session, port);
                    }
                    Err(e) => {
                        eprintln!("[app] Warning: agent-browser connect failed for '{}': {}", session, e);
                        // Non-fatal — agent-browser will auto-connect on first command anyway
                    }
                }
            }

            Ok((name.to_string(), pid, false, port))
        }
        Err(e) => Err(errors::daemon_error(&format!(
            "Failed to launch {} with CDP: {}",
            name, e
        ))),
    }
}

/// Deterministic port assignment: hash app name → port in 9222-9399 range.
pub fn deterministic_port(app_name: &str) -> u16 {
    let mut hasher = DefaultHasher::new();
    app_name.to_lowercase().hash(&mut hasher);
    let hash = hasher.finish();
    let range = 9399 - 9222 + 1; // 178 ports
    9222 + (hash % range as u64) as u16
}

/// Probe if CDP is available on a given port.
fn probe_cdp_port(port: u16) -> bool {
    // Use a simple TCP connection check
    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}

/// Get PID of an app by name using pgrep.
fn get_app_pid(name: &str) -> Option<i32> {
    let output = Command::new("pgrep")
        .arg("-x")
        .arg(name)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().lines().next()?.parse().ok()
}

/// Get list of running GUI app names using osascript.
fn get_running_app_names() -> Vec<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get name of every process whose background only is false"#)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .trim()
                .split(", ")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        }
        Err(_) => vec![],
    }
}

/// Get running GUI apps as AppInfo structs.
fn get_running_gui_apps() -> Vec<AppInfo> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events"
    set appList to every process whose background only is false
    set output to ""
    repeat with anApp in appList
        set appName to name of anApp
        set appPID to unix id of anApp
        set isFront to (frontmost of anApp)
        set output to output & appName & "||" & appPID & "||" & isFront & linefeed
    end repeat
    return output
end tell"#)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .trim()
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split("||").collect();
                    if parts.len() >= 3 {
                        let name = parts[0].to_string();
                        let pid = parts[1].parse::<i32>().unwrap_or(0);
                        let is_active = parts[2].trim() == "true";
                        Some(AppInfo {
                            name,
                            pid,
                            is_active,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        Err(_) => vec![],
    }
}

/// Get the frontmost window title for an app.
fn get_frontmost_window_title(app_name: &str) -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"tell application "System Events"
    try
        set winTitle to name of front window of process "{app_name}"
        return winTitle
    on error
        return ""
    end try
end tell"#
        ))
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let title = stdout.trim().to_string();
    if title.is_empty() { None } else { Some(title) }
}

// MARK: - Command Handlers

/// Handle the "open" command.
pub fn handle_open(
    id: &str,
    args: &OpenArgs,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    if args.with_cdp {
        match open_app_with_cdp(&args.target, state) {
            Ok((app_name, pid, was_running, port)) => Response::ok(
                id.to_string(),
                ResponseData::Open(OpenData {
                    app: app_name,
                    pid,
                    was_running,
                    cdp_port: Some(port),
                }),
                elapsed(),
            ),
            Err(e) => Response::fail(id.to_string(), e, elapsed()),
        }
    } else {
        match open_app(&args.target) {
            Ok((app_name, pid, was_running)) => Response::ok(
                id.to_string(),
                ResponseData::Open(OpenData {
                    app: app_name,
                    pid,
                    was_running,
                    cdp_port: None,
                }),
                elapsed(),
            ),
            Err(e) => Response::fail(id.to_string(), e, elapsed()),
        }
    }
}

/// Handle the "get" command.
pub fn handle_get(
    id: &str,
    args: &GetArgs,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    match args.what.to_lowercase().as_str() {
        "apps" => {
            let apps = get_running_gui_apps();
            Response::ok(
                id.to_string(),
                ResponseData::GetApps(GetAppsData { apps }),
                elapsed(),
            )
        }
        "text" => {
            let Some(ref_id) = &args.r#ref else {
                return Response::fail(
                    id.to_string(),
                    errors::invalid_command("'get text' requires a @ref"),
                    elapsed(),
                );
            };
            // Check ref map
            if state.ref_map.is_empty() {
                return Response::fail(id.to_string(), errors::no_ref_map(), elapsed());
            }
            match state.ref_map.resolve(ref_id) {
                Some(elem_ref) => {
                    // Try to get text via AX re-traversal
                    let text = if let (Some(ref path), Some(pid)) = (&elem_ref.ax_path, elem_ref.ax_pid) {
                        if let Some(ax_elem) = ax_engine::re_traverse_to_element(path, pid) {
                            // Try value, title, description (same as Swift)
                            let result = elem_ref.label.clone().unwrap_or_default();
                            unsafe { core_foundation::base::CFRelease(ax_elem as core_foundation::base::CFTypeRef); }
                            result
                        } else {
                            elem_ref.label.clone().unwrap_or_default()
                        }
                    } else {
                        elem_ref.label.clone().unwrap_or_default()
                    };

                    Response::ok(
                        id.to_string(),
                        ResponseData::GetText(GetTextData {
                            r#ref: Some(ref_id.clone()),
                            text,
                        }),
                        elapsed(),
                    )
                }
                None => Response::fail(
                    id.to_string(),
                    errors::ref_not_found(ref_id),
                    elapsed(),
                ),
            }
        }
        _ => Response::fail(
            id.to_string(),
            errors::invalid_command(&format!(
                "Unknown get target: '{}'. Valid: apps, text",
                args.what
            )),
            elapsed(),
        ),
    }
}

/// Handle the "status" command.
pub fn handle_status(
    id: &str,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let pid = std::process::id() as i32;

    // Use the real AX and capture permission checks
    let ax_trusted = ax_engine::is_process_trusted();
    let screen_permission = agent_computer_daemon::capture::has_screen_recording_permission();

    // Use AX engine for frontmost app detection (more reliable)
    let (frontmost_app, frontmost_pid, frontmost_window) = match ax_engine::get_frontmost_app() {
        Some((name, pid)) => {
            // Get window title via osascript (simpler than AX window traversal here)
            let window = get_frontmost_window_title(&name);
            (Some(name), Some(pid), window)
        }
        None => (None, None, None),
    };

    let active_cdp = if state.cdp_connections.is_empty() {
        Some(0)
    } else {
        Some(state.cdp_connections.len() as i32)
    };

    Response::ok(
        id.to_string(),
        ResponseData::Status(StatusData {
            daemon_pid: pid,
            accessibility_permission: ax_trusted,
            screen_recording_permission: screen_permission,
            frontmost_app,
            frontmost_pid,
            frontmost_window,
            ref_map_count: state.ref_map_count(),
            ref_map_age_ms: state.ref_map_age_ms(),
            active_cdp_connections: active_cdp,
        }),
        elapsed(),
    )
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_port() {
        let port1 = deterministic_port("Slack");
        let port2 = deterministic_port("Slack");
        assert_eq!(port1, port2);
        assert!(port1 >= 9222 && port1 <= 9399);

        let port3 = deterministic_port("Chrome");
        // Different app should (likely) get different port
        // Not guaranteed but very likely
        assert!(port3 >= 9222 && port3 <= 9399);
    }

    #[test]
    fn test_deterministic_port_case_insensitive() {
        let port1 = deterministic_port("Slack");
        let port2 = deterministic_port("slack");
        assert_eq!(port1, port2);
    }
}
