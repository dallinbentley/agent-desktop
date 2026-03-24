use agent_desktop_shared::protocol::*;
use agent_desktop_shared::types::ErrorInfo;

// MARK: - ANSI Colors

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

// MARK: - Public API

/// Format and print response. Returns true if successful.
pub fn print_response(response: &Response, json_mode: bool) -> bool {
    if json_mode {
        return print_json(response);
    }

    if response.success {
        if let Some(ref data) = response.data {
            print_data(data);
            return true;
        }
        return true;
    }

    if let Some(ref error) = response.error {
        print_error(error);
        return false;
    }

    print_error(&ErrorInfo {
        code: "UNKNOWN".to_string(),
        message: "Command failed with no error details.".to_string(),
        suggestion: None,
    });
    false
}

/// Print a connection/local error (not from daemon).
pub fn print_local_error(message: &str, suggestion: Option<&str>) {
    eprintln!("{RED}{BOLD}Error{RESET}{RED}: {message}{RESET}");
    if let Some(sug) = suggestion {
        eprintln!("{YELLOW}Suggestion: {sug}{RESET}");
    }
}

// MARK: - JSON Mode

fn print_json(response: &Response) -> bool {
    match serde_json::to_string_pretty(response) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize response: {}", e),
    }
    response.success
}

// MARK: - Human-Readable Output

fn print_data(data: &ResponseData) {
    match data {
        ResponseData::Snapshot(d) => print_snapshot(d),
        ResponseData::Click(d) => print_click(d),
        ResponseData::Fill(d) => print_fill(d),
        ResponseData::Type(d) => print_type(d),
        ResponseData::Press(d) => print_press(d),
        ResponseData::Scroll(d) => print_scroll(d),
        ResponseData::Screenshot(d) => print_screenshot(d),
        ResponseData::Open(d) => print_open(d),
        ResponseData::GetApps(d) => print_get_apps(d),
        ResponseData::GetText(d) => print_get_text(d),
        ResponseData::Status(d) => print_status(d),
        ResponseData::Wait(d) => print_wait(d),
    }
}

fn print_snapshot(data: &SnapshotData) {
    // The text tree is already formatted by the daemon — print as-is
    println!("{}", data.text);

    // Print profiling data to stderr when --verbose is active
    if let Some(ref profile) = data.profile {
        eprint!("{}", profile);
    }
}

fn print_click(data: &ClickData) {
    let mut parts = vec!["Clicked".to_string()];
    if let Some(ref r) = data.r#ref {
        parts.push(format!("{CYAN}@{r}{RESET}"));
    }
    if let Some(ref elem) = data.element {
        let role_short = elem.role.replace("AX", "").to_lowercase();
        parts.push(role_short);
        if let Some(ref label) = elem.label {
            parts.push(format!("{BOLD}\"{label}\"{RESET}"));
        }
    }
    let coords = format!(
        "({}, {})",
        data.coordinates.x as i32, data.coordinates.y as i32
    );
    parts.push(format!("at {DIM}{coords}{RESET}"));
    println!("{}", parts.join(" "));
}

fn print_fill(data: &FillData) {
    println!(
        "Filled {CYAN}@{}{RESET} with {BOLD}\"{}\"{RESET}",
        data.r#ref, data.text
    );
}

fn print_type(data: &TypeData) {
    if let Some(ref r) = data.r#ref {
        println!(
            "Typed {BOLD}\"{}\"{RESET} into {CYAN}@{r}{RESET}",
            data.text
        );
    } else {
        println!("Typed {BOLD}\"{}\"{RESET}", data.text);
    }
}

fn print_press(data: &PressData) {
    let mut key_combo = data.modifiers.join("+");
    if !key_combo.is_empty() {
        key_combo.push('+');
    }
    key_combo.push_str(&data.key);
    println!("Pressed {BOLD}{key_combo}{RESET}");
}

fn print_scroll(data: &ScrollData) {
    println!(
        "Scrolled {BOLD}{}{RESET} by {} pixels",
        data.direction, data.amount
    );
}

fn print_screenshot(data: &ScreenshotData) {
    println!(
        "Screenshot saved to {CYAN}{}{RESET} ({}×{})",
        data.path, data.width, data.height
    );
}

fn print_open(data: &OpenData) {
    if data.was_running {
        println!(
            "Activated {BOLD}{}{RESET} (pid {})",
            data.app, data.pid
        );
    } else {
        println!(
            "Launched {BOLD}{}{RESET} (pid {})",
            data.app, data.pid
        );
    }
    if let Some(port) = data.cdp_port {
        println!("  CDP enabled on port {CYAN}{port}{RESET}");
    }
}

fn print_get_apps(data: &GetAppsData) {
    for app in &data.apps {
        let active_marker = if app.is_active {
            format!(" {GREEN}●{RESET}")
        } else {
            String::new()
        };
        println!(
            "{BOLD}{}{RESET} (pid {}){active_marker}",
            app.name, app.pid
        );
    }
    if data.apps.is_empty() {
        println!("{DIM}No running GUI applications found.{RESET}");
    }
}

fn print_get_text(data: &GetTextData) {
    if let Some(ref r) = data.r#ref {
        println!("{CYAN}@{r}{RESET}: {}", data.text);
    } else {
        println!("{}", data.text);
    }
}

fn print_wait(data: &WaitData) {
    println!("Waited {BOLD}{}ms{RESET}", data.waited_ms);
}

fn print_status(data: &StatusData) {
    println!("{BOLD}agent-desktop daemon{RESET}");
    println!("  PID: {}", data.daemon_pid);

    let ax_status = if data.accessibility_permission {
        format!("{GREEN}✅ granted{RESET}")
    } else {
        format!("{RED}❌ denied{RESET}")
    };
    println!("  Accessibility: {ax_status}");

    let screen_status = if data.screen_recording_permission {
        format!("{GREEN}✅ granted{RESET}")
    } else {
        format!("{RED}❌ denied{RESET}")
    };
    println!("  Screen Recording: {screen_status}");

    if let Some(ref app) = data.frontmost_app {
        let mut front_line = format!("  Frontmost App: {BOLD}{app}{RESET}");
        if let Some(pid) = data.frontmost_pid {
            front_line.push_str(&format!(" (pid {pid})"));
        }
        println!("{front_line}");
        if let Some(ref window) = data.frontmost_window {
            println!("  Frontmost Window: {window}");
        }
    }

    print!("  Ref Map: {} elements", data.ref_map_count);
    if let Some(age) = data.ref_map_age_ms {
        let age_sec = age / 1000.0;
        println!(" (age: {:.1}s)", age_sec);
    } else {
        println!(" (no snapshot taken)");
    }

    if let Some(cdp) = data.active_cdp_connections {
        println!("  CDP Connections: {cdp}");
    }
}

// MARK: - Error Output

fn print_error(error: &ErrorInfo) {
    eprintln!(
        "{RED}{BOLD}Error{RESET}{RED} [{}]: {}{RESET}",
        error.code, error.message
    );
    if let Some(ref suggestion) = error.suggestion {
        eprintln!("{YELLOW}Suggestion: {suggestion}{RESET}");
    }
}
