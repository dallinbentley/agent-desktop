use agent_computer_shared::protocol::*;
use agent_computer_shared::errors;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::Command;

use agent_computer_daemon::ax_engine;
use crate::DaemonState;

// MARK: - App Management

/// Open or focus an application. Uses `open -a` for simplicity and reliability.
/// If `background` is true, launches hidden without stealing focus.
pub fn open_app(name: &str) -> Result<(String, i32, bool), agent_computer_shared::types::ErrorInfo> {
    open_app_with_options(name, false)
}

/// Open app in background (hidden, no focus steal).
pub fn open_app_background(name: &str) -> Result<(String, i32, bool), agent_computer_shared::types::ErrorInfo> {
    open_app_with_options(name, true)
}

fn open_app_with_options(name: &str, background: bool) -> Result<(String, i32, bool), agent_computer_shared::types::ErrorInfo> {
    // Check if already running via NSWorkspace native API
    let was_running = agent_computer_daemon::ns_workspace::is_app_running(name);

    // Use `open -a` to open. Add flags for background mode.
    let mut cmd = Command::new("open");
    if background {
        cmd.arg("-gj"); // -g = don't bring to front, -j = launch hidden
    }
    cmd.arg("-a").arg(name);
    let result = cmd.output();

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
/// Force-quits existing instance (waits for PID to exit), then relaunches
/// with --remote-debugging-port. Waits for new PID + CDP port readiness.
pub fn open_app_with_cdp(
    name: &str,
    state: &mut DaemonState,
) -> Result<(String, i32, bool, u16), agent_computer_shared::types::ErrorInfo> {
    let port = deterministic_port(name);

    // Task 5.1: Check if app is already running and get its PID
    let old_pid = get_app_pid(name);

    if let Some(old_pid) = old_pid {
        eprintln!("[app] {} is running (PID {}), quitting before relaunch...", name, old_pid);

        // Send quit via osascript
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(format!(r#"tell application "{}" to quit"#, name))
            .output();

        // Loop-wait until old PID is gone (100ms intervals, 5s timeout)
        let mut exited = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Check if PID is still in the process table
            let check = Command::new("kill")
                .arg("-0")
                .arg(old_pid.to_string())
                .output();
            match check {
                Ok(output) if !output.status.success() => {
                    // Process is gone
                    exited = true;
                    break;
                }
                _ => continue,
            }
        }

        if !exited {
            // Force kill if graceful quit didn't work
            eprintln!("[app] {} didn't quit gracefully, force killing PID {}...", name, old_pid);
            let _ = Command::new("kill")
                .arg("-9")
                .arg(old_pid.to_string())
                .output();
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Remove old PID from cdp_port_map
        state.cdp_port_map.remove(&old_pid);
    }

    // Relaunch with CDP flag
    // Note: some apps (e.g. Spotify) force-activate on launch — this is app-specific
    // behavior we can't override. The one-time focus steal is acceptable since all
    // subsequent interactions are fully headless via --app.
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

            // Task 5.2: Wait for new PID to appear (100ms intervals, 10s timeout)
            let mut new_pid: Option<i32> = None;
            for _ in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Some(pid) = get_app_pid(name) {
                    // Make sure it's a new PID (not the old one lingering)
                    if old_pid.map_or(true, |old| pid != old) {
                        new_pid = Some(pid);
                        break;
                    }
                }
            }

            let pid = new_pid.unwrap_or_else(|| {
                eprintln!("[app] Warning: couldn't detect new PID for {}", name);
                get_app_pid(name).unwrap_or(0)
            });

            // Task 5.2: Probe CDP port until responsive (100ms intervals, 10s timeout)
            let mut cdp_ready = false;
            for _ in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if probe_cdp_port(port) {
                    cdp_ready = true;
                    break;
                }
            }

            if !cdp_ready {
                eprintln!(
                    "Warning: {} launched but CDP not responding on port {}",
                    name, port
                );
            }

            // Track in state
            state.cdp_connections.insert(name.to_string(), port);
            state.port_assignments.insert(name.to_string(), port);

            // Task 5.2: Store in cdp_port_map for PID-based routing
            if pid != 0 {
                state.cdp_port_map.insert(pid, (port, name.to_string()));
                eprintln!("[app] Stored CDP mapping: PID {} → port {} ({})", pid, port, name);
            }

            // Auto-connect agent-browser session
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

/// Get PID of an app by name using NSWorkspace native API.
/// Falls back to pgrep for apps that may have different process names.
fn get_app_pid(name: &str) -> Option<i32> {
    // Try NSWorkspace first (fast, no subprocess)
    if let Some(pid) = agent_computer_daemon::ns_workspace::get_app_pid_by_name(name) {
        return Some(pid);
    }
    // Fallback to pgrep for apps whose process name differs from display name
    let output = Command::new("pgrep")
        .arg("-x")
        .arg(name)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().lines().next()?.parse().ok()
}

/// Get list of running GUI app names using NSWorkspace native API.
fn get_running_app_names() -> Vec<String> {
    agent_computer_daemon::ns_workspace::get_running_app_names()
}

/// Get running GUI apps as AppInfo structs using NSWorkspace native API.
fn get_running_gui_apps() -> Vec<AppInfo> {
    agent_computer_daemon::ns_workspace::get_running_gui_apps()
}

/// Get the frontmost window title for an app using AX API.
fn get_frontmost_window_title_for_pid(pid: i32) -> Option<String> {
    ax_engine::get_window_title_for_pid(pid)
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
        let result = if args.background {
            open_app_background(&args.target)
        } else {
            open_app(&args.target)
        };
        match result {
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
                    // Task 8.2: If ref is CDP-sourced, delegate to browser_bridge.get_web()
                    if elem_ref.source == agent_computer_shared::types::RefSource::CDP {
                        if let (Some(ref ab_ref), Some(ref session), Some(cdp_port)) =
                            (&elem_ref.ab_ref, &elem_ref.ab_session, elem_ref.cdp_port)
                        {
                            match state.browser_bridge.get_web(session, cdp_port, "text", Some(ab_ref)) {
                                Ok(text) => {
                                    return Response::ok(
                                        id.to_string(),
                                        ResponseData::GetText(GetTextData {
                                            r#ref: Some(ref_id.clone()),
                                            text: text.trim().to_string(),
                                        }),
                                        elapsed(),
                                    );
                                }
                                Err(e) => {
                                    eprintln!("[get] CDP get_web failed, falling back to label: {}", e);
                                    // Fall through to label-based approach below
                                }
                            }
                        }
                    }

                    // AX-based text retrieval
                    let text = if let (Some(ref path), Some(pid)) = (&elem_ref.ax_path, elem_ref.ax_pid) {
                        if let Some(ax_elem) = ax_engine::re_traverse_to_element(path, pid) {
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
        "title" | "url" => {
            // Task 8.2: title/url via CDP — these don't require a ref
            let what = args.what.to_lowercase();

            // Determine CDP session/port
            let (session, port) = if let Some(ref app_name) = args.app {
                let session = app_name.to_lowercase().replace(' ', "-");
                let port = state.get_cdp_port_for_app(app_name);
                (Some(session), port)
            } else if state.last_snapshot_cdp_sourced {
                (state.last_cdp_session.clone(), state.last_cdp_port)
            } else {
                (None, None)
            };

            if let (Some(session), Some(port)) = (session, port) {
                let ab_ref = args.r#ref.as_ref().map(|r| {
                    // Look up agent-browser ref from ref map
                    state.ref_map.resolve(r)
                        .and_then(|e| e.ab_ref.clone())
                        .unwrap_or_else(|| r.clone())
                });
                match state.browser_bridge.get_web(&session, port, &what, ab_ref.as_deref()) {
                    Ok(text) => {
                        Response::ok(
                            id.to_string(),
                            ResponseData::GetText(GetTextData {
                                r#ref: args.r#ref.clone(),
                                text: text.trim().to_string(),
                            }),
                            elapsed(),
                        )
                    }
                    Err(e) => Response::fail(
                        id.to_string(),
                        errors::cdp_error(&format!("get {} failed: {}", what, e)),
                        elapsed(),
                    ),
                }
            } else {
                Response::fail(
                    id.to_string(),
                    errors::invalid_command(&format!(
                        "'get {}' requires a CDP app (use --app or snapshot a CDP app first)",
                        what
                    )),
                    elapsed(),
                )
            }
        }
        _ => Response::fail(
            id.to_string(),
            errors::invalid_command(&format!(
                "Unknown get target: '{}'. Valid: apps, text, title, url",
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
            // Get window title via AX API (native, no subprocess)
            let window = get_frontmost_window_title_for_pid(pid);
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
