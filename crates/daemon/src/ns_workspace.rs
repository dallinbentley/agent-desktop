// NSWorkspace native API module — replaces osascript subprocess calls
// for app discovery with direct Cocoa API calls via objc2.

use agent_computer_shared::protocol::AppInfo;
use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};

/// Get all running GUI applications using NSWorkspace.shared.runningApplications.
/// Filters to apps with activationPolicy == .regular (visible GUI apps only).
/// Returns the same data as the old osascript approach: name, PID, is_active.
pub fn get_running_gui_apps() -> Vec<AppInfo> {
    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();

    let mut result = Vec::new();
    for app in apps.iter() {
        // Only include regular (GUI) apps — skip background & accessory apps
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let name = app
            .localizedName()
            .map(|n| n.to_string())
            .unwrap_or_default();

        if name.is_empty() {
            continue;
        }

        let pid = app.processIdentifier();
        let is_active = app.isActive();

        result.push(AppInfo {
            name,
            pid,
            is_active,
        });
    }

    result
}

/// Get list of running GUI app names (convenience wrapper).
pub fn get_running_app_names() -> Vec<String> {
    get_running_gui_apps()
        .into_iter()
        .map(|app| app.name)
        .collect()
}

/// Check if an app with the given name is currently running.
/// Returns the PID if found.
pub fn get_app_pid_by_name(name: &str) -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();

    for app in apps.iter() {
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }

        let app_name = app.localizedName().map(|n| n.to_string());
        if let Some(ref app_name) = app_name {
            if app_name == name {
                return Some(app.processIdentifier());
            }
        }
    }

    None
}

/// Check if an app with the given name is running (bool convenience).
pub fn is_app_running(name: &str) -> bool {
    get_app_pid_by_name(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_running_gui_apps_smoke() {
        // Should not crash, and should return at least Finder (always running)
        let apps = get_running_gui_apps();
        // Finder is always running on macOS
        let has_finder = apps.iter().any(|a| a.name == "Finder");
        assert!(has_finder, "Finder should always be in running apps");
        // All PIDs should be positive
        for app in &apps {
            assert!(app.pid > 0, "PID should be positive for {}", app.name);
        }
    }

    #[test]
    fn test_get_running_app_names_smoke() {
        let names = get_running_app_names();
        assert!(names.contains(&"Finder".to_string()));
    }

    #[test]
    fn test_is_app_running() {
        // Finder is always running
        assert!(is_app_running("Finder"));
        // This app definitely doesn't exist
        assert!(!is_app_running("NonExistentApp12345"));
    }
}
