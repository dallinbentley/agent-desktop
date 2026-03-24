// CDP Engine — Minimal port probing utilities
// The full CDP WebSocket engine has been removed in favor of the agent-browser bridge.
// Only port probing remains (used by detector.rs for app kind detection).

use std::io::Read as _;
use std::time::Duration;

use serde::Deserialize;

// MARK: - CDP Types (minimal, for port probing only)

/// CDP browser version info from /json/version
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CdpVersionInfo {
    #[serde(rename = "Browser")]
    pub browser: Option<String>,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: Option<String>,
    #[serde(rename = "User-Agent")]
    pub user_agent: Option<String>,
    #[serde(rename = "V8-Version")]
    pub v8_version: Option<String>,
    #[serde(rename = "WebKit-Version")]
    pub webkit_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: Option<String>,
}

/// Probe result from a CDP port
#[derive(Debug, Clone)]
pub struct CdpProbeResult {
    pub available: bool,
    pub port: u16,
    pub version_info: Option<CdpVersionInfo>,
}

// MARK: - CDP Port Probing

/// Probe a single CDP port with 500ms timeout
pub fn probe_cdp_port(port: u16) -> CdpProbeResult {
    let url = format!("http://localhost:{port}/json/version");
    match ureq::get(&url).timeout(Duration::from_millis(500)).call() {
        Ok(resp) => {
            let mut body = String::new();
            if resp.into_reader().read_to_string(&mut body).is_ok() {
                if let Ok(info) = serde_json::from_str::<CdpVersionInfo>(&body) {
                    return CdpProbeResult {
                        available: true,
                        port,
                        version_info: Some(info),
                    };
                }
            }
            CdpProbeResult {
                available: false,
                port,
                version_info: None,
            }
        }
        Err(_) => CdpProbeResult {
            available: false,
            port,
            version_info: None,
        },
    }
}

/// Scan standard CDP ports 9222-9229 and return the first available
pub fn scan_cdp_ports() -> Option<CdpProbeResult> {
    for port in 9222..=9229 {
        let result = probe_cdp_port(port);
        if result.available {
            return Some(result);
        }
    }
    None
}

/// Probe a specific port, or scan standard range
pub fn find_cdp_port(specific_port: Option<u16>) -> Option<CdpProbeResult> {
    if let Some(port) = specific_port {
        let result = probe_cdp_port(port);
        if result.available {
            return Some(result);
        }
        None
    } else {
        scan_cdp_ports()
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_nonexistent_port() {
        let result = probe_cdp_port(19999);
        assert!(!result.available);
        assert!(result.version_info.is_none());
    }
}
