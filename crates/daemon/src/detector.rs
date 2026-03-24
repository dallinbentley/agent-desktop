// App Detector & Routing — detects app kind and routes snapshots/interactions
// Tasks 7.1-7.3

use std::path::{Path, PathBuf};

use agent_computer_shared::types::{AppKind, KNOWN_BROWSER_BUNDLE_IDS};

use crate::refmap::InteractionRoute;

// MARK: - 7.1 App Detection

/// Detect the kind of application from its PID (task 7.1).
///
/// Detection priority:
/// 1. Check bundle ID against known browsers → Browser
/// 2. Check for Electron Framework.framework → Electron
/// 3. Check for Chromium Embedded Framework.framework → CEF
/// 4. Otherwise → Native
///
/// For Browser/Electron/CEF: probe for CDP port
pub fn detect_app(pid: i32, bundle_id: Option<&str>, bundle_path: Option<&Path>) -> AppKind {
    // Step 1: Check known browser bundle IDs
    if let Some(bid) = bundle_id {
        if KNOWN_BROWSER_BUNDLE_IDS.contains(bid) {
            let cdp_port = probe_app_cdp_port(pid, None);
            return AppKind::Browser { cdp_port };
        }
    }

    // Step 2 & 3: Check bundle contents for Electron or CEF frameworks
    if let Some(path) = bundle_path {
        if has_electron_framework(path) {
            let cdp_port = probe_app_cdp_port(pid, None);
            return AppKind::Electron { cdp_port };
        }
        if has_cef_framework(path) {
            let cdp_port = probe_app_cdp_port(pid, None);
            return AppKind::CEF { cdp_port };
        }
    }

    // Step 4: Native app
    AppKind::Native
}

/// Check if a bundle path contains the Electron framework
fn has_electron_framework(bundle_path: &Path) -> bool {
    let electron_path = bundle_path
        .join("Contents")
        .join("Frameworks")
        .join("Electron Framework.framework");
    electron_path.exists()
}

/// Check if a bundle path contains the CEF (Chromium Embedded Framework)
fn has_cef_framework(bundle_path: &Path) -> bool {
    // CEF apps may have CEF framework in various locations
    let cef_paths = [
        bundle_path
            .join("Contents")
            .join("Frameworks")
            .join("Chromium Embedded Framework.framework"),
        // Some apps put it in a helper location
        bundle_path
            .join("Contents")
            .join("Frameworks")
            .join("Chromium Embedded Framework"),
    ];
    cef_paths.iter().any(|p| p.exists())
}

/// Probe a single CDP port by hitting /json/version with 500ms timeout.
/// Returns true if a CDP endpoint is responding on this port.
fn probe_cdp_port(port: u16) -> bool {
    let url = format!("http://localhost:{port}/json/version");
    ureq::get(&url)
        .timeout(std::time::Duration::from_millis(500))
        .call()
        .is_ok()
}

/// Scan standard CDP ports 9222-9229 and return the first available.
fn scan_cdp_ports() -> Option<u16> {
    for port in 9222..=9229 {
        if probe_cdp_port(port) {
            return Some(port);
        }
    }
    None
}

/// Probe for a CDP port on an app.
/// If a specific port is given, try that first.
/// Otherwise scan standard ports 9222-9229.
fn probe_app_cdp_port(_pid: i32, specific_port: Option<u16>) -> Option<u16> {
    // Try deterministic port first (based on app)
    if let Some(port) = specific_port {
        if probe_cdp_port(port) {
            return Some(port);
        }
    }

    // Scan standard range
    if let Some(port) = scan_cdp_ports() {
        // TODO: Verify the CDP port actually belongs to this PID
        return Some(port);
    }

    None
}

/// Get the bundle path for a macOS app from its PID.
/// Uses /proc or lsof to find the executable, then walks up to .app
pub fn get_bundle_path_for_pid(pid: i32) -> Option<PathBuf> {
    // On macOS, we can use `lsof -p PID` or read from /proc equivalent
    // For now, use ps to get the executable path
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;

    let comm = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if comm.is_empty() {
        return None;
    }

    // Try to find the .app bundle by walking up from the executable
    let path = Path::new(&comm);
    let mut current: Option<&Path> = Some(path);
    while let Some(p) = current {
        if p.extension().and_then(|e| e.to_str()) == Some("app") {
            return Some(p.to_path_buf());
        }
        current = p.parent();
    }

    // Alternative: try the /Applications path
    // Many macOS apps live under /Applications/AppName.app
    None
}

/// Get the bundle ID for a macOS app from its PID using mdls or defaults
pub fn get_bundle_id_for_pid(pid: i32) -> Option<String> {
    // Use `lsappinfo info -only bundleid <pid>` on macOS
    let output = std::process::Command::new("lsappinfo")
        .args(["info", "-only", "bundleid", &format!("{pid}")])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse output like: "bundleid"="com.google.Chrome"
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("\"bundleid\"=") {
            let bid = rest.trim().trim_matches('"');
            if !bid.is_empty() {
                return Some(bid.to_string());
            }
        }
    }

    None
}

/// Full detection: given just a PID, detect the app kind
pub fn detect_app_from_pid(pid: i32) -> AppKind {
    let bundle_id = get_bundle_id_for_pid(pid);
    let bundle_path = get_bundle_path_for_pid(pid);

    detect_app(
        pid,
        bundle_id.as_deref(),
        bundle_path.as_deref(),
    )
}

// MARK: - 7.2 Snapshot Routing

/// Determines how to take a snapshot based on the app kind (task 7.2)
#[derive(Debug, Clone)]
pub enum SnapshotStrategy {
    /// Native app: AX engine only
    AXOnly,
    /// Browser with CDP: AX for chrome (stop at AXWebArea), CDP for web content, merge
    MergedAXAndCDP { cdp_port: u16 },
    /// Electron/CEF with CDP: CDP only (skip AX)
    CDPOnly { cdp_port: u16 },
    /// No CDP available: AX-only fallback with warning
    AXFallback { reason: String },
    /// Screenshot fallback (no AX or CDP)
    ScreenshotFallback { reason: String },
}

/// Determine the snapshot strategy for an app (task 7.2)
pub fn snapshot_strategy(app_kind: &AppKind) -> SnapshotStrategy {
    match app_kind {
        AppKind::Native => SnapshotStrategy::AXOnly,

        AppKind::Browser { cdp_port } => match cdp_port {
            Some(port) => SnapshotStrategy::MergedAXAndCDP { cdp_port: *port },
            None => SnapshotStrategy::AXFallback {
                reason: "Browser has no CDP port. Use `open --with-cdp` to enable rich web content access.".to_string(),
            },
        },

        AppKind::Electron { cdp_port } => match cdp_port {
            Some(port) => SnapshotStrategy::CDPOnly { cdp_port: *port },
            None => SnapshotStrategy::AXFallback {
                reason: "Electron app has no CDP port. Use `open --with-cdp` to enable rich UI access.".to_string(),
            },
        },

        AppKind::CEF { cdp_port } => match cdp_port {
            Some(port) => SnapshotStrategy::CDPOnly { cdp_port: *port },
            None => SnapshotStrategy::AXFallback {
                reason: "CEF app has no CDP port. Use `open --with-cdp` to enable rich UI access.".to_string(),
            },
        },

        AppKind::Unknown => SnapshotStrategy::AXFallback {
            reason: "Unknown app type, using accessibility fallback.".to_string(),
        },
    }
}

// MARK: - 7.3 Interaction Routing

/// Determines how to interact with an element (task 7.3)
#[derive(Debug)]
pub enum InteractionEngine {
    /// Use AX engine (native accessibility actions)
    AX,
    /// Use agent-browser bridge (headless CDP via agent-browser CLI)
    AgentBrowser {
        session: String,
        cdp_port: u16,
        ab_ref: String,
    },
    /// Use input engine (CGEvent-based coordinate input)
    Input { x: f64, y: f64 },
}

/// Route an interaction based on the resolved ref (task 7.3)
pub fn route_interaction(route: &InteractionRoute) -> InteractionEngine {
    match route {
        InteractionRoute::AX { .. } => InteractionEngine::AX,
        InteractionRoute::AgentBrowser {
            session,
            cdp_port,
            ab_ref,
            ..
        } => InteractionEngine::AgentBrowser {
            session: session.clone(),
            cdp_port: *cdp_port,
            ab_ref: ab_ref.clone(),
        },
        InteractionRoute::Coordinate { x, y, .. } => InteractionEngine::Input { x: *x, y: *y },
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_known_browser() {
        let kind = detect_app(1234, Some("com.google.Chrome"), None);
        match kind {
            AppKind::Browser { .. } => {} // expected
            other => panic!("Expected Browser, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_safari() {
        let kind = detect_app(1234, Some("com.apple.Safari"), None);
        match kind {
            AppKind::Browser { .. } => {}
            other => panic!("Expected Browser, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_arc() {
        let kind = detect_app(1234, Some("company.thebrowser.Browser"), None);
        match kind {
            AppKind::Browser { .. } => {}
            other => panic!("Expected Browser, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_native_app() {
        let kind = detect_app(1234, Some("com.apple.finder"), None);
        match kind {
            AppKind::Native => {}
            other => panic!("Expected Native, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_unknown_bundle() {
        let kind = detect_app(1234, Some("com.unknown.app"), None);
        match kind {
            AppKind::Native => {} // Unknown bundle ID falls through to path checks, then Native
            other => panic!("Expected Native, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_no_info() {
        let kind = detect_app(1234, None, None);
        match kind {
            AppKind::Native => {}
            other => panic!("Expected Native, got {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_strategy_native() {
        let kind = AppKind::Native;
        match snapshot_strategy(&kind) {
            SnapshotStrategy::AXOnly => {}
            other => panic!("Expected AXOnly, got {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_strategy_browser_with_cdp() {
        let kind = AppKind::Browser {
            cdp_port: Some(9222),
        };
        match snapshot_strategy(&kind) {
            SnapshotStrategy::MergedAXAndCDP { cdp_port } => {
                assert_eq!(cdp_port, 9222);
            }
            other => panic!("Expected MergedAXAndCDP, got {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_strategy_browser_no_cdp() {
        let kind = AppKind::Browser { cdp_port: None };
        match snapshot_strategy(&kind) {
            SnapshotStrategy::AXFallback { reason } => {
                assert!(reason.contains("CDP"));
            }
            other => panic!("Expected AXFallback, got {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_strategy_electron_with_cdp() {
        let kind = AppKind::Electron {
            cdp_port: Some(9223),
        };
        match snapshot_strategy(&kind) {
            SnapshotStrategy::CDPOnly { cdp_port } => {
                assert_eq!(cdp_port, 9223);
            }
            other => panic!("Expected CDPOnly, got {:?}", other),
        }
    }

    #[test]
    fn test_snapshot_strategy_cef_no_cdp() {
        let kind = AppKind::CEF { cdp_port: None };
        match snapshot_strategy(&kind) {
            SnapshotStrategy::AXFallback { reason } => {
                assert!(reason.contains("CEF"));
            }
            other => panic!("Expected AXFallback, got {:?}", other),
        }
    }

    #[test]
    fn test_has_electron_framework() {
        // This will be false for non-existent paths
        let fake_path = Path::new("/tmp/FakeApp.app");
        assert!(!has_electron_framework(fake_path));
    }

    #[test]
    fn test_has_cef_framework() {
        let fake_path = Path::new("/tmp/FakeApp.app");
        assert!(!has_cef_framework(fake_path));
    }

    #[test]
    fn test_interaction_routing() {
        use crate::refmap::InteractionRoute;
        use agent_computer_shared::types::{ElementRef, RefSource};

        let ax_route = InteractionRoute::AX {
            pid: 123,
            element: ElementRef {
                id: "e1".to_string(),
                source: RefSource::AX,
                role: "button".to_string(),
                label: None,
                frame: None,
                ax_path: None,
                ax_actions: None,
                ax_pid: Some(123),
                cdp_node_id: None,
                cdp_backend_node_id: None,
                cdp_port: None,
                ab_ref: None,
                ab_session: None,
            },
        };
        match route_interaction(&ax_route) {
            InteractionEngine::AX => {}
            _ => panic!("Expected AX engine"),
        }

        let ab_route = InteractionRoute::AgentBrowser {
            session: "spotify".to_string(),
            cdp_port: 9222,
            ab_ref: "e32".to_string(),
            element: ElementRef {
                id: "e2".to_string(),
                source: RefSource::CDP,
                role: "link".to_string(),
                label: None,
                frame: None,
                ax_path: None,
                ax_actions: None,
                ax_pid: None,
                cdp_node_id: Some(10),
                cdp_backend_node_id: Some(42),
                cdp_port: Some(9222),
                ab_ref: Some("e32".to_string()),
                ab_session: Some("spotify".to_string()),
            },
        };
        match route_interaction(&ab_route) {
            InteractionEngine::AgentBrowser {
                session,
                cdp_port,
                ab_ref,
            } => {
                assert_eq!(cdp_port, 9222);
                assert_eq!(ab_ref, "e32");
                assert_eq!(session, "spotify");
            }
            _ => panic!("Expected AgentBrowser engine"),
        }
    }
}
