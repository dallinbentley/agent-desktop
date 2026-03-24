// Unified RefMap — dual-source element reference storage
// Tasks 9.1-9.3

use std::collections::HashMap;
use std::time::Instant;

use agent_computer_shared::types::{ElementRef, RefSource};

// MARK: - Routing Info

/// Routing info for dispatching interactions to the correct engine
#[derive(Debug, Clone)]
pub enum InteractionRoute {
    /// Route to AX engine (native accessibility)
    AX {
        pid: i32,
        element: ElementRef,
    },
    /// Route to agent-browser bridge (web/Electron content via CDP)
    AgentBrowser {
        session: String,
        cdp_port: u16,
        ab_ref: String,
        element: ElementRef,
    },
    /// Route to input engine (coordinate-based)
    Coordinate {
        x: f64,
        y: f64,
        element: ElementRef,
    },
}

// MARK: - 9.1 RefMap

/// Stores ElementRef entries keyed by ref ID ("e1", "e2", ...).
/// Each entry has a RefSource (AX or CDP) and source-specific data.
pub struct RefMap {
    entries: HashMap<String, ElementRef>,
    created_at: Instant,
}

impl RefMap {
    /// Create a new empty RefMap
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            created_at: Instant::now(),
        }
    }

    /// Insert a single element ref
    pub fn insert(&mut self, element: ElementRef) {
        self.entries.insert(element.id.clone(), element);
    }

    /// Resolve a ref ID to its ElementRef (task 9.1)
    pub fn resolve(&self, ref_id: &str) -> Option<&ElementRef> {
        // Accept both "e1" and "@e1" formats
        let clean_id = ref_id.strip_prefix('@').unwrap_or(ref_id);
        self.entries.get(clean_id)
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Age of this ref map in milliseconds
    pub fn age_ms(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64() * 1000.0
    }

    /// Clear all entries and reset timestamp
    pub fn clear(&mut self) {
        self.entries.clear();
        self.created_at = Instant::now();
    }

    /// Get all entries
    pub fn entries(&self) -> &HashMap<String, ElementRef> {
        &self.entries
    }

    /// Get all entries as a sorted vec (by ref number)
    pub fn sorted_entries(&self) -> Vec<&ElementRef> {
        let mut entries: Vec<&ElementRef> = self.entries.values().collect();
        entries.sort_by(|a, b| {
            let num_a = a.id.strip_prefix('e').and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            let num_b = b.id.strip_prefix('e').and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            num_a.cmp(&num_b)
        });
        entries
    }

    // MARK: - 9.3 Source-Aware Dispatch

    /// Resolve a ref and determine the interaction route (task 9.3)
    pub fn route(&self, ref_id: &str) -> Result<InteractionRoute, String> {
        let element = self
            .resolve(ref_id)
            .ok_or_else(|| format!("Ref @{ref_id} not found"))?;

        match element.source {
            RefSource::AX => {
                let pid = element
                    .ax_pid
                    .ok_or_else(|| format!("AX ref @{ref_id} has no PID"))?;
                Ok(InteractionRoute::AX {
                    pid,
                    element: element.clone(),
                })
            }
            RefSource::CDP => {
                // CDP-sourced refs are now routed through agent-browser bridge
                let port = element
                    .cdp_port
                    .ok_or_else(|| format!("CDP ref @{ref_id} has no port"))?;
                let ab_ref = element
                    .ab_ref
                    .as_ref()
                    .ok_or_else(|| format!("CDP ref @{ref_id} has no agent-browser ref"))?
                    .clone();
                let session = element
                    .ab_session
                    .as_ref()
                    .ok_or_else(|| format!("CDP ref @{ref_id} has no agent-browser session"))?
                    .clone();
                Ok(InteractionRoute::AgentBrowser {
                    session,
                    cdp_port: port,
                    ab_ref,
                    element: element.clone(),
                })
            }
            RefSource::Coordinate => {
                let (x, y) = element
                    .center()
                    .ok_or_else(|| format!("Coordinate ref @{ref_id} has no frame"))?;
                Ok(InteractionRoute::Coordinate {
                    x,
                    y,
                    element: element.clone(),
                })
            }
        }
    }
}

// MARK: - 9.2 Merged Ref Building

/// Build a merged RefMap from AX nodes and CDP nodes (task 9.2).
/// AX refs come first (browser chrome), then CDP refs (web content).
/// Continuous numbering: @e1, @e2, ... across both sources.
pub fn build_merged_refmap(
    ax_refs: Vec<ElementRef>,
    cdp_refs: Vec<ElementRef>,
) -> RefMap {
    let mut refmap = RefMap::new();
    let mut counter: usize = 1;

    // AX refs first (browser chrome like address bar, tabs, toolbar)
    for mut elem in ax_refs {
        elem.id = format!("e{counter}");
        refmap.insert(elem);
        counter += 1;
    }

    // CDP refs second (web page content)
    for mut elem in cdp_refs {
        elem.id = format!("e{counter}");
        refmap.insert(elem);
        counter += 1;
    }

    refmap
}

/// Build a RefMap from only AX refs (native apps)
pub fn build_ax_refmap(ax_refs: Vec<ElementRef>) -> RefMap {
    build_merged_refmap(ax_refs, vec![])
}

/// Build a RefMap from only CDP refs (Electron/CEF apps)
pub fn build_cdp_refmap(cdp_refs: Vec<ElementRef>) -> RefMap {
    build_merged_refmap(vec![], cdp_refs)
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;
    use agent_computer_shared::types::Rect;

    fn make_ax_ref(id: &str, role: &str, label: Option<&str>, pid: i32) -> ElementRef {
        ElementRef {
            id: id.to_string(),
            source: RefSource::AX,
            role: role.to_string(),
            label: label.map(|s| s.to_string()),
            frame: Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 30.0,
            }),
            ax_path: None,
            ax_actions: Some(vec!["AXPress".to_string()]),
            ax_pid: Some(pid),
            cdp_node_id: None,
            cdp_backend_node_id: None,
            cdp_port: None,
            ab_ref: None,
            ab_session: None,
        }
    }

    fn make_cdp_ref(id: &str, role: &str, label: Option<&str>, port: u16) -> ElementRef {
        ElementRef {
            id: id.to_string(),
            source: RefSource::CDP,
            role: role.to_string(),
            label: label.map(|s| s.to_string()),
            frame: None,
            ax_path: None,
            ax_actions: None,
            ax_pid: None,
            cdp_node_id: Some(42),
            cdp_backend_node_id: Some(100),
            cdp_port: Some(port),
            ab_ref: Some(format!("e{}", 50)), // original agent-browser ref
            ab_session: Some("test-app".to_string()),
        }
    }

    #[test]
    fn test_refmap_basic_operations() {
        let mut refmap = RefMap::new();
        assert!(refmap.is_empty());

        refmap.insert(make_ax_ref("e1", "button", Some("OK"), 1234));
        assert_eq!(refmap.len(), 1);

        let elem = refmap.resolve("e1");
        assert!(elem.is_some());
        assert_eq!(elem.unwrap().role, "button");
        assert_eq!(elem.unwrap().label.as_deref(), Some("OK"));

        // Test @-prefix stripping
        let elem2 = refmap.resolve("@e1");
        assert!(elem2.is_some());
        assert_eq!(elem2.unwrap().id, "e1");
    }

    #[test]
    fn test_refmap_resolve_missing() {
        let refmap = RefMap::new();
        assert!(refmap.resolve("e99").is_none());
    }

    #[test]
    fn test_merged_refmap_numbering() {
        let ax_refs = vec![
            make_ax_ref("tmp1", "button", Some("Back"), 123),
            make_ax_ref("tmp2", "textbox", Some("Address"), 123),
        ];
        let cdp_refs = vec![
            make_cdp_ref("tmp3", "link", Some("Home"), 9222),
            make_cdp_ref("tmp4", "button", Some("Submit"), 9222),
            make_cdp_ref("tmp5", "textbox", Some("Search"), 9222),
        ];

        let refmap = build_merged_refmap(ax_refs, cdp_refs);
        assert_eq!(refmap.len(), 5);

        // AX refs get e1, e2
        let e1 = refmap.resolve("e1").unwrap();
        assert_eq!(e1.source, RefSource::AX);
        assert_eq!(e1.label.as_deref(), Some("Back"));

        let e2 = refmap.resolve("e2").unwrap();
        assert_eq!(e2.source, RefSource::AX);
        assert_eq!(e2.label.as_deref(), Some("Address"));

        // CDP refs get e3, e4, e5
        let e3 = refmap.resolve("e3").unwrap();
        assert_eq!(e3.source, RefSource::CDP);
        assert_eq!(e3.label.as_deref(), Some("Home"));

        let e4 = refmap.resolve("e4").unwrap();
        assert_eq!(e4.source, RefSource::CDP);
        assert_eq!(e4.label.as_deref(), Some("Submit"));

        let e5 = refmap.resolve("e5").unwrap();
        assert_eq!(e5.source, RefSource::CDP);
        assert_eq!(e5.label.as_deref(), Some("Search"));
    }

    #[test]
    fn test_routing_ax_ref() {
        let mut refmap = RefMap::new();
        refmap.insert(make_ax_ref("e1", "button", Some("OK"), 1234));

        let route = refmap.route("e1").unwrap();
        match route {
            InteractionRoute::AX { pid, element } => {
                assert_eq!(pid, 1234);
                assert_eq!(element.role, "button");
            }
            _ => panic!("Expected AX route"),
        }
    }

    #[test]
    fn test_routing_cdp_ref() {
        let mut refmap = RefMap::new();
        refmap.insert(make_cdp_ref("e1", "link", Some("Home"), 9222));

        let route = refmap.route("e1").unwrap();
        match route {
            InteractionRoute::AgentBrowser {
                session,
                cdp_port,
                ab_ref,
                element,
            } => {
                assert_eq!(cdp_port, 9222);
                assert_eq!(ab_ref, "e50");
                assert_eq!(session, "test-app");
                assert_eq!(element.role, "link");
            }
            _ => panic!("Expected AgentBrowser route"),
        }
    }

    #[test]
    fn test_routing_coordinate_ref() {
        let mut refmap = RefMap::new();
        refmap.insert(ElementRef {
            id: "e1".to_string(),
            source: RefSource::Coordinate,
            role: "button".to_string(),
            label: Some("Target".to_string()),
            frame: Some(Rect {
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 30.0,
            }),
            ax_path: None,
            ax_actions: None,
            ax_pid: None,
            cdp_node_id: None,
            cdp_backend_node_id: None,
            cdp_port: None,
            ab_ref: None,
            ab_session: None,
        });

        let route = refmap.route("e1").unwrap();
        match route {
            InteractionRoute::Coordinate { x, y, .. } => {
                assert!((x - 125.0).abs() < 0.1); // center = 100 + 50/2
                assert!((y - 215.0).abs() < 0.1); // center = 200 + 30/2
            }
            _ => panic!("Expected Coordinate route"),
        }
    }

    #[test]
    fn test_routing_missing_ref() {
        let refmap = RefMap::new();
        assert!(refmap.route("e99").is_err());
    }

    #[test]
    fn test_sorted_entries() {
        let ax_refs = vec![
            make_ax_ref("tmp", "button", Some("B"), 1),
            make_ax_ref("tmp", "link", Some("A"), 1),
        ];
        let cdp_refs = vec![make_cdp_ref("tmp", "textbox", Some("C"), 9222)];
        let refmap = build_merged_refmap(ax_refs, cdp_refs);

        let sorted = refmap.sorted_entries();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].id, "e1");
        assert_eq!(sorted[1].id, "e2");
        assert_eq!(sorted[2].id, "e3");
    }

    #[test]
    fn test_refmap_clear() {
        let mut refmap = RefMap::new();
        refmap.insert(make_ax_ref("e1", "button", Some("OK"), 1234));
        assert_eq!(refmap.len(), 1);

        refmap.clear();
        assert!(refmap.is_empty());
        assert!(refmap.resolve("e1").is_none());
    }

    #[test]
    fn test_ax_only_refmap() {
        let refs = vec![
            make_ax_ref("tmp", "button", Some("OK"), 1),
            make_ax_ref("tmp", "textbox", Some("Name"), 1),
        ];
        let refmap = build_ax_refmap(refs);
        assert_eq!(refmap.len(), 2);
        assert_eq!(refmap.resolve("e1").unwrap().source, RefSource::AX);
        assert_eq!(refmap.resolve("e2").unwrap().source, RefSource::AX);
    }

    #[test]
    fn test_cdp_only_refmap() {
        let refs = vec![
            make_cdp_ref("tmp", "link", Some("Home"), 9222),
            make_cdp_ref("tmp", "button", Some("Login"), 9222),
        ];
        let refmap = build_cdp_refmap(refs);
        assert_eq!(refmap.len(), 2);
        assert_eq!(refmap.resolve("e1").unwrap().source, RefSource::CDP);
        assert_eq!(refmap.resolve("e2").unwrap().source, RefSource::CDP);
    }
}
