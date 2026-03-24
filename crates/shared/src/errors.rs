use crate::types::{ErrorInfo, error_code};

/// Factory functions for AI-friendly error messages
pub fn ref_not_found(ref_id: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::REF_NOT_FOUND.to_string(),
        message: format!("Element @{ref_id} not found. The UI may have changed."),
        suggestion: Some("Run `snapshot` to refresh element references.".to_string()),
    }
}

pub fn ref_stale(ref_id: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::REF_STALE.to_string(),
        message: format!("Element @{ref_id} existed but can no longer be located."),
        suggestion: Some("Run `snapshot` to refresh element references.".to_string()),
    }
}

pub fn no_ref_map() -> ErrorInfo {
    ErrorInfo {
        code: error_code::NO_REF_MAP.to_string(),
        message: "No element references available.".to_string(),
        suggestion: Some("Run `snapshot -i` first to discover interactive elements.".to_string()),
    }
}

pub fn app_not_found(name: &str, suggestions: &[String]) -> ErrorInfo {
    let msg = if suggestions.is_empty() {
        format!("Application '{name}' not found.")
    } else {
        format!("Application '{name}' not found. Running apps: {}", suggestions.join(", "))
    };
    ErrorInfo {
        code: error_code::APP_NOT_FOUND.to_string(),
        message: msg,
        suggestion: Some("Check the app name and try again.".to_string()),
    }
}

pub fn permission_denied_accessibility() -> ErrorInfo {
    ErrorInfo {
        code: error_code::PERMISSION_DENIED.to_string(),
        message: "Accessibility permission required.".to_string(),
        suggestion: Some("Grant access in System Settings → Privacy & Security → Accessibility.".to_string()),
    }
}

pub fn permission_denied_screen_recording() -> ErrorInfo {
    ErrorInfo {
        code: error_code::PERMISSION_DENIED.to_string(),
        message: "Screen Recording permission required.".to_string(),
        suggestion: Some("Grant access in System Settings → Privacy & Security → Screen Recording.".to_string()),
    }
}

pub fn timeout(partial_count: usize, total_estimate: usize) -> ErrorInfo {
    ErrorInfo {
        code: error_code::TIMEOUT.to_string(),
        message: format!("Snapshot timed out. Returning partial results ({partial_count} of ~{total_estimate} elements)."),
        suggestion: Some("Try `snapshot -d 5` to reduce depth.".to_string()),
    }
}

pub fn ax_error(detail: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::AX_ERROR.to_string(),
        message: format!("Accessibility API error: {detail}"),
        suggestion: Some("This may be a transient issue. Try again, or check that the target app is responsive.".to_string()),
    }
}

pub fn input_error(detail: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::INPUT_ERROR.to_string(),
        message: format!("Input simulation error: {detail}"),
        suggestion: Some("Check that Accessibility permission is granted.".to_string()),
    }
}

pub fn invalid_command(detail: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::INVALID_COMMAND.to_string(),
        message: format!("Invalid command: {detail}"),
        suggestion: Some("Run `agent-computer --help` for usage.".to_string()),
    }
}

pub fn daemon_error(detail: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::DAEMON_ERROR.to_string(),
        message: format!("Daemon error: {detail}"),
        suggestion: None,
    }
}

pub fn cdp_not_available(app_name: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::CDP_NOT_AVAILABLE.to_string(),
        message: format!("{app_name} does not have CDP (Chrome DevTools Protocol) enabled."),
        suggestion: Some(format!("Run `agent-computer open --with-cdp {app_name}` to relaunch with rich UI support.")),
    }
}

pub fn cdp_error(detail: &str) -> ErrorInfo {
    ErrorInfo {
        code: error_code::CDP_ERROR.to_string(),
        message: format!("CDP error: {detail}"),
        suggestion: Some("Check that the browser/app is running and CDP port is accessible.".to_string()),
    }
}
