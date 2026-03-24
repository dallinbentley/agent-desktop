use agent_computer_shared::protocol::*;
use agent_computer_shared::types::*;
use agent_computer_shared::errors;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::sync::Mutex;

mod app;
mod browser_bridge;
mod detector;
mod refmap;

// Re-export lib crate modules for use in this binary
use agent_computer_daemon::ax_engine;
use agent_computer_daemon::capture;
use agent_computer_daemon::input;

// MARK: - Daemon State

/// Shared daemon state accessible by all handlers.
pub struct DaemonState {
    /// Unified ref map
    pub ref_map: refmap::RefMap,
    /// Active CDP connections: app_name -> port
    pub cdp_connections: HashMap<String, u16>,
    /// Deterministic port assignments: app_name -> port
    pub port_assignments: HashMap<String, u16>,
    /// Browser bridge for agent-browser subprocess management
    pub browser_bridge: browser_bridge::BrowserBridge,
    /// Track whether the last snapshot was CDP-sourced (for press/scroll delegation)
    pub last_snapshot_cdp_sourced: bool,
    /// Session/port of last CDP-sourced snapshot (for press/scroll)
    pub last_cdp_session: Option<String>,
    pub last_cdp_port: Option<u16>,
}

impl DaemonState {
    fn new() -> Self {
        let bridge = browser_bridge::BrowserBridge::new();
        if bridge.is_available() {
            log("agent-browser detected and available");
        } else {
            log("WARNING: agent-browser not found. Web/Electron support disabled. Install with: npm install -g agent-browser");
        }
        Self {
            ref_map: refmap::RefMap::new(),
            cdp_connections: HashMap::new(),
            port_assignments: HashMap::new(),
            browser_bridge: bridge,
            last_snapshot_cdp_sourced: false,
            last_cdp_session: None,
            last_cdp_port: None,
        }
    }

    pub fn ref_map_count(&self) -> i32 {
        self.ref_map.len() as i32
    }

    pub fn ref_map_age_ms(&self) -> Option<f64> {
        if self.ref_map.is_empty() {
            None
        } else {
            Some(self.ref_map.age_ms())
        }
    }
}

// MARK: - Logging

fn log(msg: &str) {
    let now = chrono::Local::now();
    eprintln!("[{}] {}", now.format("%H:%M:%S%.3f"), msg);
}

// MARK: - Command Dispatch (Task 11.1)

/// Central dispatch function — routes commands to real engine implementations.
pub fn handle_command(
    command: &str,
    args: &serde_json::Value,
    id: &str,
    state: &mut DaemonState,
) -> Response {
    let start = std::time::Instant::now();
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    match command {
        "snapshot" => handle_snapshot(id, args, state, start),
        "click" => handle_click(id, args, state, start),
        "fill" => handle_fill(id, args, state, start),
        "type" => handle_type(id, args, state, start),
        "press" => handle_press(id, args, state, start),
        "scroll" => handle_scroll(id, args, state, start),
        "screenshot" => handle_screenshot(id, args, start),
        "open" => {
            let open_args: OpenArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("open requires args with target"),
                        elapsed(),
                    );
                }
            };
            app::handle_open(id, &open_args, state, start)
        }
        "get" => {
            let get_args: GetArgs = match serde_json::from_value(args.clone()) {
                Ok(a) => a,
                Err(_) => {
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command("get requires args with what"),
                        elapsed(),
                    );
                }
            };
            app::handle_get(id, &get_args, state, start)
        }
        "status" => app::handle_status(id, state, start),
        _ => Response::fail(
            id.to_string(),
            errors::invalid_command(&format!("Unknown command: '{}'", command)),
            elapsed(),
        ),
    }
}

// MARK: - Snapshot Command (detect → route → AX/CDP/merged → format → respond)

fn handle_snapshot(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let snap_args: SnapshotArgs = serde_json::from_value(args.clone()).unwrap_or(SnapshotArgs {
        interactive: true,
        compact: false,
        depth: None,
        app: None,
    });

    let depth = snap_args.depth.unwrap_or(20);

    // Get frontmost app or specified app
    let (app_name, pid) = if let Some(ref target_app) = snap_args.app {
        // Find the app by name — use osascript to get PID
        match get_app_pid_by_name(target_app) {
            Some((name, pid)) => (name, pid),
            None => {
                return Response::fail(
                    id.to_string(),
                    errors::app_not_found(target_app, &[]),
                    elapsed(),
                );
            }
        }
    } else {
        // Use frontmost app
        match ax_engine::get_frontmost_app() {
            Some((name, pid)) => (name, pid),
            None => {
                return Response::fail(
                    id.to_string(),
                    errors::daemon_error("Could not determine frontmost application"),
                    elapsed(),
                );
            }
        }
    };

    // Detect app kind and determine snapshot strategy
    let app_kind = detector::detect_app_from_pid(pid);
    let strategy = detector::snapshot_strategy(&app_kind);

    match strategy {
        detector::SnapshotStrategy::AXOnly | detector::SnapshotStrategy::AXFallback { .. } => {
            // AX-only snapshot
            let (tree, ax_app_name, window_title) = ax_engine::take_snapshot(pid, depth, 3.0);
            let display_name = if ax_app_name != "Unknown" { ax_app_name } else { app_name };

            let (text, refs) = ax_engine::format_snapshot_text(
                &tree,
                &display_name,
                &window_title,
                snap_args.interactive,
                pid,
            );

            let ref_count = refs.len() as i32;

            // Update ref map
            state.ref_map.clear();
            for r in refs {
                state.ref_map.insert(r);
            }
            state.last_snapshot_cdp_sourced = false;

            Response::ok(
                id.to_string(),
                ResponseData::Snapshot(SnapshotData {
                    text,
                    ref_count,
                    app: display_name,
                    window: window_title,
                }),
                elapsed(),
            )
        }
        detector::SnapshotStrategy::CDPOnly { cdp_port } => {
            // CDP-only snapshot via agent-browser bridge (Electron/CEF apps)
            if !state.browser_bridge.is_available() {
                log("agent-browser not available, falling back to AX for CDP-only app");
                let (tree, ax_name, win_title) = ax_engine::take_snapshot(pid, depth, 3.0);
                let name = if ax_name != "Unknown" { ax_name } else { app_name };
                let (text, refs) = ax_engine::format_snapshot_text(&tree, &name, &win_title, snap_args.interactive, pid);
                let ref_count = refs.len() as i32;
                state.ref_map.clear();
                for r in refs { state.ref_map.insert(r); }
                state.last_snapshot_cdp_sourced = false;
                return Response::ok(id.to_string(), ResponseData::Snapshot(SnapshotData { text, ref_count, app: name, window: win_title }), elapsed());
            }

            let session = app_name.to_lowercase().replace(' ', "-");
            match state.browser_bridge.snapshot(&session, cdp_port, snap_args.interactive) {
                Ok(parsed_elements) => {
                    // Build ElementRefs from parsed agent-browser output
                    let mut refs = Vec::new();
                    let mut lines = Vec::new();
                    let mut counter: usize = 1;

                    for elem in &parsed_elements {
                        let ref_id = format!("e{counter}");
                        let line = if let Some(ref lbl) = elem.label {
                            format!("  @{ref_id} {} \"{}\"", elem.role, lbl)
                        } else {
                            format!("  @{ref_id} {}", elem.role)
                        };
                        lines.push(line);

                        refs.push(ElementRef {
                            id: ref_id,
                            source: RefSource::CDP,
                            role: elem.role.clone(),
                            label: elem.label.clone(),
                            frame: None,
                            ax_path: None,
                            ax_actions: None,
                            ax_pid: None,
                            cdp_node_id: None,
                            cdp_backend_node_id: None,
                            cdp_port: Some(cdp_port),
                            ab_ref: Some(elem.ref_id.clone()),
                            ab_session: Some(session.clone()),
                        });
                        counter += 1;
                    }

                    let ref_count = refs.len() as i32;
                    let text = format!("[{}]\n{}", app_name, lines.join("\n"));

                    state.ref_map.clear();
                    for r in refs { state.ref_map.insert(r); }
                    state.last_snapshot_cdp_sourced = true;
                    state.last_cdp_session = Some(session);
                    state.last_cdp_port = Some(cdp_port);

                    Response::ok(
                        id.to_string(),
                        ResponseData::Snapshot(SnapshotData {
                            text,
                            ref_count,
                            app: app_name,
                            window: None,
                        }),
                        elapsed(),
                    )
                }
                Err(e) => {
                    // Fall back to AX
                    log(&format!("agent-browser snapshot failed, falling back to AX: {}", e));
                    let (tree, ax_name, win_title) = ax_engine::take_snapshot(pid, depth, 3.0);
                    let name = if ax_name != "Unknown" { ax_name } else { app_name };
                    let (text, refs) = ax_engine::format_snapshot_text(&tree, &name, &win_title, snap_args.interactive, pid);
                    let ref_count = refs.len() as i32;
                    state.ref_map.clear();
                    for r in refs { state.ref_map.insert(r); }
                    state.last_snapshot_cdp_sourced = false;
                    Response::ok(id.to_string(), ResponseData::Snapshot(SnapshotData { text, ref_count, app: name, window: win_title }), elapsed())
                }
            }
        }
        detector::SnapshotStrategy::MergedAXAndCDP { cdp_port } => {
            // Merged: AX for browser chrome (stop at AXWebArea), agent-browser for web content
            let (tree, ax_name, win_title) = ax_engine::take_snapshot(pid, depth, 3.0);
            let display_name = if ax_name != "Unknown" { ax_name } else { app_name.clone() };

            let (ax_text, ax_refs) = ax_engine::format_snapshot_text(
                &tree, &display_name, &win_title, snap_args.interactive, pid,
            );

            // Try to get web content via agent-browser
            let session = app_name.to_lowercase().replace(' ', "-");
            let (merged_text, all_refs, cdp_sourced) = if state.browser_bridge.is_available() {
                match state.browser_bridge.snapshot(&session, cdp_port, snap_args.interactive) {
                    Ok(parsed_elements) => {
                        let mut text = ax_text;
                        let ax_count = ax_refs.len();
                        let mut all = ax_refs;

                        if !parsed_elements.is_empty() {
                            text.push_str("  --- web content ---\n");
                            let mut counter = ax_count + 1;
                            for elem in &parsed_elements {
                                let ref_id = format!("e{counter}");
                                let line = if let Some(ref lbl) = elem.label {
                                    format!("  @{ref_id} {} \"{}\"", elem.role, lbl)
                                } else {
                                    format!("  @{ref_id} {}", elem.role)
                                };
                                text.push_str(&line);
                                text.push('\n');

                                all.push(ElementRef {
                                    id: ref_id,
                                    source: RefSource::CDP,
                                    role: elem.role.clone(),
                                    label: elem.label.clone(),
                                    frame: None,
                                    ax_path: None,
                                    ax_actions: None,
                                    ax_pid: None,
                                    cdp_node_id: None,
                                    cdp_backend_node_id: None,
                                    cdp_port: Some(cdp_port),
                                    ab_ref: Some(elem.ref_id.clone()),
                                    ab_session: Some(session.clone()),
                                });
                                counter += 1;
                            }
                        }
                        (text, all, true)
                    }
                    Err(e) => {
                        log(&format!("agent-browser snapshot failed in merged mode: {}", e));
                        (ax_text, ax_refs, false)
                    }
                }
            } else {
                log("agent-browser not available for merged snapshot, using AX only");
                (ax_text, ax_refs, false)
            };

            let ref_count = all_refs.len() as i32;
            state.ref_map.clear();
            for r in all_refs { state.ref_map.insert(r); }
            state.last_snapshot_cdp_sourced = cdp_sourced;
            if cdp_sourced {
                state.last_cdp_session = Some(session);
                state.last_cdp_port = Some(cdp_port);
            }

            Response::ok(
                id.to_string(),
                ResponseData::Snapshot(SnapshotData {
                    text: merged_text,
                    ref_count,
                    app: display_name,
                    window: win_title,
                }),
                elapsed(),
            )
        }
        detector::SnapshotStrategy::ScreenshotFallback { reason } => {
            // Screenshot-based fallback
            Response::fail(
                id.to_string(),
                errors::daemon_error(&format!(
                    "Snapshot not available: {}. Use `screenshot` command instead.",
                    reason
                )),
                elapsed(),
            )
        }
    }
}

// MARK: - Click Command (resolve ref → dispatch to correct engine) (Task 11.1 + 11.2)

fn handle_click(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let click_args: ClickArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("click requires args with ref or x/y"),
                elapsed(),
            );
        }
    };

    let click_count = if click_args.double { 2u32 } else { 1u32 };
    let button = if click_args.right {
        input::MouseButton::Right
    } else {
        input::MouseButton::Left
    };

    // Coordinate-based click — requires --foreground when targeting a background app
    if let (Some(x), Some(y)) = (click_args.x, click_args.y) {
        if click_args.app.is_some() && !click_args.foreground {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("Coordinate clicks require --foreground flag or the app must be frontmost."),
                elapsed(),
            );
        }
        // If --foreground specified, bring app to front first
        if click_args.foreground {
            if let Some(ref app_name) = click_args.app {
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(format!(r#"tell application "{}" to activate"#, app_name))
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        input::mouse_click(x, y, button, click_count);
        return Response::ok(
            id.to_string(),
            ResponseData::Click(ClickData {
                r#ref: None,
                coordinates: Point { x, y },
                element: None,
            }),
            elapsed(),
        );
    }

    // Ref-based click
    let ref_id = match &click_args.r#ref {
        Some(r) => r.clone(),
        None => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("click requires ref or x/y coordinates"),
                elapsed(),
            );
        }
    };

    if state.ref_map.is_empty() {
        return Response::fail(id.to_string(), errors::no_ref_map(), elapsed());
    }

    let route = match state.ref_map.route(&ref_id) {
        Ok(r) => r,
        Err(_) => {
            return Response::fail(id.to_string(), errors::ref_not_found(&ref_id), elapsed());
        }
    };

    match route {
        refmap::InteractionRoute::AX { pid, element } => {
            // Fallback chain (Task 11.2): try AX action first → CGEvent fallback
            let mut clicked_via_ax = false;

            // Try AX press action
            if let Some(ref actions) = element.ax_actions {
                if actions.iter().any(|a| a == "AXPress") {
                    if let Some(ref path) = element.ax_path {
                        if let Some(ax_elem) = ax_engine::re_traverse_to_element(path, pid) {
                            if ax_engine::ax_press(ax_elem).is_ok() {
                                clicked_via_ax = true;
                            }
                            unsafe {
                                core_foundation::base::CFRelease(ax_elem as core_foundation::base::CFTypeRef);
                            }
                        }
                    }
                }
            }

            // CGEvent fallback — only if not in headless --app mode
            if !clicked_via_ax {
                if click_args.app.is_some() && !click_args.foreground {
                    // In --app mode without --foreground, CGEvent clicks would steal focus
                    return Response::fail(
                        id.to_string(),
                        errors::invalid_command(
                            "AX headless click failed and CGEvent fallback requires --foreground in --app mode."
                        ),
                        elapsed(),
                    );
                }
                if let Some((cx, cy)) = element.center() {
                    // Bring to front if --foreground was specified
                    if click_args.foreground {
                        if let Some(ref app_name) = click_args.app {
                            let _ = std::process::Command::new("osascript")
                                .arg("-e")
                                .arg(format!(r#"tell application "{}" to activate"#, app_name))
                                .output();
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                    }
                    input::mouse_click(cx, cy, button, click_count);
                } else {
                    return Response::fail(
                        id.to_string(),
                        errors::ref_stale(&ref_id),
                        elapsed(),
                    );
                }
            }

            let coords = element.center().map(|(x, y)| Point { x, y }).unwrap_or(Point { x: 0.0, y: 0.0 });
            Response::ok(
                id.to_string(),
                ResponseData::Click(ClickData {
                    r#ref: Some(ref_id),
                    coordinates: coords,
                    element: Some(ElementInfo {
                        role: element.role.clone(),
                        label: element.label.clone(),
                    }),
                }),
                elapsed(),
            )
        }
        refmap::InteractionRoute::AgentBrowser {
            session,
            cdp_port,
            ab_ref,
            element,
        } => {
            // Agent-browser click — delegate to bridge
            match state.browser_bridge.click(&session, cdp_port, &ab_ref) {
                Ok(_) => {
                    let coords = element.center().map(|(x, y)| Point { x, y }).unwrap_or(Point { x: 0.0, y: 0.0 });
                    Response::ok(
                        id.to_string(),
                        ResponseData::Click(ClickData {
                            r#ref: Some(ref_id),
                            coordinates: coords,
                            element: Some(ElementInfo {
                                role: element.role.clone(),
                                label: element.label.clone(),
                            }),
                        }),
                        elapsed(),
                    )
                }
                Err(e) => Response::fail(
                    id.to_string(),
                    errors::cdp_error(&e),
                    elapsed(),
                ),
            }
        }
        refmap::InteractionRoute::Coordinate { x, y, element } => {
            input::mouse_click(x, y, button, click_count);
            Response::ok(
                id.to_string(),
                ResponseData::Click(ClickData {
                    r#ref: Some(ref_id),
                    coordinates: Point { x, y },
                    element: Some(ElementInfo {
                        role: element.role.clone(),
                        label: element.label.clone(),
                    }),
                }),
                elapsed(),
            )
        }
    }
}

// MARK: - Fill Command (Task 11.2: AX set_value → selection-replace → CGEvent Cmd+A+type)

fn handle_fill(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let fill_args: FillArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("fill requires args with ref and text"),
                elapsed(),
            );
        }
    };

    if state.ref_map.is_empty() {
        return Response::fail(id.to_string(), errors::no_ref_map(), elapsed());
    }

    let route = match state.ref_map.route(&fill_args.r#ref) {
        Ok(r) => r,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::ref_not_found(&fill_args.r#ref),
                elapsed(),
            );
        }
    };

    match route {
        refmap::InteractionRoute::AX { pid, element } => {
            // Fallback chain: kAXValueAttribute → selection-replace → CGEvent Cmd+A+type
            if let Some(ref path) = element.ax_path {
                if let Some(ax_elem) = ax_engine::re_traverse_to_element(path, pid) {
                    // Try 1: AX set value
                    if ax_engine::ax_set_value(ax_elem, &fill_args.text).is_ok() {
                        unsafe { core_foundation::base::CFRelease(ax_elem as core_foundation::base::CFTypeRef); }
                        return Response::ok(
                            id.to_string(),
                            ResponseData::Fill(FillData {
                                r#ref: fill_args.r#ref,
                                text: fill_args.text,
                            }),
                            elapsed(),
                        );
                    }

                    // Try 2: Selection replace
                    if ax_engine::ax_selection_replace(ax_elem, &fill_args.text).is_ok() {
                        unsafe { core_foundation::base::CFRelease(ax_elem as core_foundation::base::CFTypeRef); }
                        return Response::ok(
                            id.to_string(),
                            ResponseData::Fill(FillData {
                                r#ref: fill_args.r#ref,
                                text: fill_args.text,
                            }),
                            elapsed(),
                        );
                    }

                    unsafe { core_foundation::base::CFRelease(ax_elem as core_foundation::base::CFTypeRef); }
                }
            }

            // Try 3: CGEvent fallback — click to focus, Cmd+A, type
            if let Some((cx, cy)) = element.center() {
                input::mouse_click(cx, cy, input::MouseButton::Left, 1);
                std::thread::sleep(std::time::Duration::from_millis(50));
                // Cmd+A to select all
                input::key_press(0, core_graphics::event::CGEventFlags::CGEventFlagCommand); // 'a' keycode = 0
                std::thread::sleep(std::time::Duration::from_millis(50));
                // Type the replacement text
                input::type_string(&fill_args.text);
            }

            Response::ok(
                id.to_string(),
                ResponseData::Fill(FillData {
                    r#ref: fill_args.r#ref,
                    text: fill_args.text,
                }),
                elapsed(),
            )
        }
        refmap::InteractionRoute::AgentBrowser {
            session,
            cdp_port,
            ab_ref,
            ..
        } => {
            // Agent-browser fill — delegate to bridge
            match state.browser_bridge.fill(&session, cdp_port, &ab_ref, &fill_args.text) {
                Ok(_) => {
                    Response::ok(
                        id.to_string(),
                        ResponseData::Fill(FillData {
                            r#ref: fill_args.r#ref,
                            text: fill_args.text,
                        }),
                        elapsed(),
                    )
                }
                Err(e) => Response::fail(id.to_string(), errors::cdp_error(&e), elapsed()),
            }
        }
        refmap::InteractionRoute::Coordinate { x, y, .. } => {
            // Click to focus, select all, type
            input::mouse_click(x, y, input::MouseButton::Left, 1);
            std::thread::sleep(std::time::Duration::from_millis(50));
            input::key_press(0, core_graphics::event::CGEventFlags::CGEventFlagCommand);
            std::thread::sleep(std::time::Duration::from_millis(50));
            input::type_string(&fill_args.text);

            Response::ok(
                id.to_string(),
                ResponseData::Fill(FillData {
                    r#ref: fill_args.r#ref,
                    text: fill_args.text,
                }),
                elapsed(),
            )
        }
    }
}

// MARK: - Type Command

fn handle_type(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let type_args: TypeArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("type requires args with text"),
                elapsed(),
            );
        }
    };

    if let Some(ref ref_id) = type_args.r#ref {
        // Type into a specific element
        if state.ref_map.is_empty() {
            return Response::fail(id.to_string(), errors::no_ref_map(), elapsed());
        }

        let route = match state.ref_map.route(ref_id) {
            Ok(r) => r,
            Err(_) => {
                return Response::fail(id.to_string(), errors::ref_not_found(ref_id), elapsed());
            }
        };

        match route {
            refmap::InteractionRoute::AX { element, .. } => {
                // Click to focus, then type
                if let Some((cx, cy)) = element.center() {
                    input::mouse_click(cx, cy, input::MouseButton::Left, 1);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                input::type_string(&type_args.text);
            }
            refmap::InteractionRoute::AgentBrowser {
                session,
                cdp_port,
                ab_ref,
                ..
            } => {
                // Agent-browser type — delegate to bridge
                if let Err(e) = state.browser_bridge.type_text(&session, cdp_port, &ab_ref, &type_args.text) {
                    log(&format!("agent-browser type_text failed: {}", e));
                }
            }
            refmap::InteractionRoute::Coordinate { x, y, .. } => {
                input::mouse_click(x, y, input::MouseButton::Left, 1);
                std::thread::sleep(std::time::Duration::from_millis(50));
                input::type_string(&type_args.text);
            }
        }
    } else {
        // Type without a target — just type
        input::type_string(&type_args.text);
    }

    Response::ok(
        id.to_string(),
        ResponseData::Type(TypeData {
            r#ref: type_args.r#ref,
            text: type_args.text,
        }),
        elapsed(),
    )
}

// MARK: - Press Command

fn handle_press(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let press_args: PressArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("press requires args with key"),
                elapsed(),
            );
        }
    };

    // If last snapshot was CDP-sourced, delegate to agent-browser for headless key press
    if state.last_snapshot_cdp_sourced {
        if let (Some(ref session), Some(cdp_port)) = (&state.last_cdp_session, state.last_cdp_port) {
            match state.browser_bridge.press(session, cdp_port, &press_args.key) {
                Ok(()) => {
                    return Response::ok(
                        id.to_string(),
                        ResponseData::Press(PressData {
                            key: press_args.key,
                            modifiers: press_args.modifiers.unwrap_or_default(),
                        }),
                        elapsed(),
                    );
                }
                Err(e) => {
                    log(&format!("agent-browser press failed, falling back to CGEvent: {}", e));
                    // Fall through to CGEvent below
                }
            }
        }
    }

    let modifiers = press_args.modifiers.as_deref().unwrap_or(&[]);
    let flags = input::parse_modifier_flags(modifiers);

    // Look up keycode
    let key_lower = press_args.key.to_lowercase();
    let keycode = match KEY_NAME_TO_CODE.get(key_lower.as_str()) {
        Some(code) => *code,
        None => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command(&format!("Unknown key: '{}'", press_args.key)),
                elapsed(),
            );
        }
    };

    input::key_press(keycode, flags);

    Response::ok(
        id.to_string(),
        ResponseData::Press(PressData {
            key: press_args.key,
            modifiers: modifiers.to_vec(),
        }),
        elapsed(),
    )
}

// MARK: - Scroll Command

fn handle_scroll(
    id: &str,
    args: &serde_json::Value,
    state: &mut DaemonState,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let scroll_args: ScrollArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(_) => {
            return Response::fail(
                id.to_string(),
                errors::invalid_command("scroll requires args with direction"),
                elapsed(),
            );
        }
    };

    let amount = scroll_args.amount.unwrap_or(3); // Default 3 lines

    // If last snapshot was CDP-sourced, delegate to agent-browser for headless scroll
    if state.last_snapshot_cdp_sourced {
        if let (Some(ref session), Some(cdp_port)) = (&state.last_cdp_session, state.last_cdp_port) {
            match state.browser_bridge.scroll(session, cdp_port, &scroll_args.direction, amount) {
                Ok(()) => {
                    return Response::ok(
                        id.to_string(),
                        ResponseData::Scroll(ScrollData {
                            direction: scroll_args.direction,
                            amount,
                        }),
                        elapsed(),
                    );
                }
                Err(e) => {
                    log(&format!("agent-browser scroll failed, falling back to CGEvent: {}", e));
                    // Fall through to CGEvent below
                }
            }
        }
    }

    input::scroll(&scroll_args.direction, amount);

    Response::ok(
        id.to_string(),
        ResponseData::Scroll(ScrollData {
            direction: scroll_args.direction,
            amount,
        }),
        elapsed(),
    )
}

// MARK: - Screenshot Command

fn handle_screenshot(
    id: &str,
    args: &serde_json::Value,
    start: std::time::Instant,
) -> Response {
    let elapsed = || start.elapsed().as_secs_f64() * 1000.0;

    let screen_args: ScreenshotArgs = serde_json::from_value(args.clone()).unwrap_or(ScreenshotArgs {
        full: false,
        app: None,
    });

    match capture::capture_screenshot(screen_args.full, screen_args.app.as_deref()) {
        Ok(result) => Response::ok(
            id.to_string(),
            ResponseData::Screenshot(ScreenshotData {
                path: result.path,
                width: result.width,
                height: result.height,
                scale: result.scale,
                window_origin_x: result.window_origin_x,
                window_origin_y: result.window_origin_y,
                app_name: result.app_name,
            }),
            elapsed(),
        ),
        Err(e) => {
            if e.contains("Screen Recording permission") {
                Response::fail(
                    id.to_string(),
                    errors::permission_denied_screen_recording(),
                    elapsed(),
                )
            } else {
                Response::fail(
                    id.to_string(),
                    errors::daemon_error(&e),
                    elapsed(),
                )
            }
        }
    }
}

// MARK: - Helpers

/// Get app PID by name using osascript.
fn get_app_pid_by_name(name: &str) -> Option<(String, i32)> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"tell application "System Events"
    set targetProc to first process whose name is "{name}"
    return (name of targetProc) & "||" & (unix id of targetProc)
end tell"#
        ))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split("||").collect();
    if parts.len() >= 2 {
        let app_name = parts[0].to_string();
        let pid = parts[1].parse::<i32>().ok()?;
        Some((app_name, pid))
    } else {
        None
    }
}

// MARK: - Stale Socket Handling

fn handle_stale_socket(path: &PathBuf) {
    if !path.exists() {
        return;
    }

    match std::os::unix::net::UnixStream::connect(path) {
        Ok(_) => {
            log("Another daemon is already running. Exiting.");
            std::process::exit(1);
        }
        Err(_) => {
            log("Stale socket found, removing...");
            let _ = std::fs::remove_file(path);
        }
    }
}

// MARK: - Client Handler

async fn handle_client(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<DaemonState>>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(json) => {
                        let id = json
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let command = json
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args = json
                            .get("args")
                            .cloned()
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                        let mut state = state.lock().await;
                        handle_command(&command, &args, &id, &mut state)
                    }
                    Err(_) => Response::fail(
                        "unknown".to_string(),
                        errors::invalid_command("Malformed JSON request"),
                        0.0,
                    ),
                };

                match serde_json::to_string(&response) {
                    Ok(json) => {
                        let mut resp_bytes = json.into_bytes();
                        resp_bytes.push(b'\n');
                        if writer.write_all(&resp_bytes).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to serialize response: {}", e));
                        break;
                    }
                }
            }
            Err(e) => {
                log(&format!("Read error: {}", e));
                break;
            }
        }
    }
}

// MARK: - Main Entry Point

#[tokio::main]
async fn main() {
    log("agent-computer-daemon starting...");
    log(&format!("PID: {}", std::process::id()));

    let socket_dir = daemon_socket_dir();
    let socket_path = daemon_socket_path();

    if let Err(e) = std::fs::create_dir_all(&socket_dir) {
        log(&format!("ERROR: Failed to create socket directory: {}", e));
        std::process::exit(1);
    }

    handle_stale_socket(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("ERROR: Failed to bind socket: {}", e));
            std::process::exit(1);
        }
    };

    log(&format!("Listening on {}", socket_path.display()));

    let state = Arc::new(Mutex::new(DaemonState::new()));

    let socket_path_clone = socket_path.clone();
    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        log("Client connected");
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            handle_client(stream, state).await;
                            log("Client disconnected");
                        });
                    }
                    Err(e) => {
                        log(&format!("Accept error: {}", e));
                    }
                }
            }
        } => {}
        _ = async {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler");
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to install SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => { log("Received SIGTERM"); }
                _ = sigint.recv() => { log("Received SIGINT"); }
            }
        } => {}
    }

    log("Shutting down...");

    // Close all agent-browser sessions (Task 5.3)
    {
        let mut state = state.lock().await;
        let session_count = state.browser_bridge.active_sessions.len();
        if session_count > 0 {
            log(&format!("Closing {} agent-browser session(s)...", session_count));
            state.browser_bridge.close_all();
            log("All agent-browser sessions closed");
        }
    }

    let _ = std::fs::remove_file(&socket_path_clone);
    log("Daemon exited cleanly");
}
