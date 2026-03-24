// AX Engine — Accessibility tree traversal, interactive filtering,
// headless actions, and frontmost app detection.
//
// Uses raw FFI to ApplicationServices for AXUIElement APIs,
// plus core-foundation for CF types.

use agent_desktop_shared::types::{ElementRef, PathSegment, Rect, RefSource, INTERACTIVE_ROLES};
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFTypeID, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;
use std::time::{Duration, Instant};

// ============================================================================
// MARK: - Snapshot Profiling
// ============================================================================

/// Per-depth-level statistics collected during tree traversal.
#[derive(Debug, Clone)]
pub struct DepthStats {
    pub depth: usize,
    pub duration: Duration,
    pub element_count: usize,
}

/// Profiling data collected during a snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotProfile {
    /// Per-depth-level timing and element counts.
    pub depth_stats: Vec<DepthStats>,
    /// Total number of AX attribute queries made.
    pub total_ax_queries: usize,
    /// Total number of elements visited.
    pub total_elements: usize,
    /// Total wall-clock duration of the snapshot.
    pub total_duration: Duration,
}

impl SnapshotProfile {
    fn new() -> Self {
        Self {
            depth_stats: Vec::new(),
            total_ax_queries: 0,
            total_elements: 0,
            total_duration: Duration::ZERO,
        }
    }

    /// Ensure the depth_stats vec has an entry for this depth.
    fn ensure_depth(&mut self, depth: usize) {
        while self.depth_stats.len() <= depth {
            let d = self.depth_stats.len();
            self.depth_stats.push(DepthStats {
                depth: d,
                duration: Duration::ZERO,
                element_count: 0,
            });
        }
    }

    /// Record time and element for a given depth.
    fn record(&mut self, depth: usize, duration: Duration) {
        self.ensure_depth(depth);
        self.depth_stats[depth].duration += duration;
        self.depth_stats[depth].element_count += 1;
        self.total_elements += 1;
    }

    /// Increment the AX attribute query counter.
    fn count_queries(&mut self, n: usize) {
        self.total_ax_queries += n;
    }

    /// Format profiling data for stderr output.
    pub fn format_report(&self) -> String {
        let mut out = String::new();
        out.push_str("\n[snapshot profiling]\n");
        out.push_str(&format!(
            "  total: {:.1}ms | {} elements | {} AX queries\n",
            self.total_duration.as_secs_f64() * 1000.0,
            self.total_elements,
            self.total_ax_queries,
        ));
        out.push_str("  per-depth breakdown:\n");
        for ds in &self.depth_stats {
            if ds.element_count > 0 {
                let avg_us = if ds.element_count > 0 {
                    ds.duration.as_micros() as f64 / ds.element_count as f64
                } else {
                    0.0
                };
                out.push_str(&format!(
                    "    depth {:2}: {:6.1}ms | {:5} elements | {:.0}µs/elem\n",
                    ds.depth,
                    ds.duration.as_secs_f64() * 1000.0,
                    ds.element_count,
                    avg_us,
                ));
            }
        }
        out
    }
}

// ============================================================================
// MARK: - Raw FFI to ApplicationServices
// ============================================================================

pub type AXUIElementRef = *mut c_void;
pub type AXValueRef = *mut c_void;
pub type AXError = i32;

// AXError codes
pub const K_AX_ERROR_SUCCESS: AXError = 0;
#[allow(dead_code)]
pub const K_AX_ERROR_ATTRIBUTE_UNSUPPORTED: AXError = -25205;

// AXValue types
pub const K_AX_VALUE_TYPE_CG_POINT: i32 = 1;
pub const K_AX_VALUE_TYPE_CG_SIZE: i32 = 2;
#[allow(dead_code)]
pub const K_AX_VALUE_TYPE_CG_RECT: i32 = 3;
pub const K_AX_VALUE_TYPE_CF_RANGE: i32 = 4;

// AXUIElement batch option
pub const K_AX_COPY_MULTIPLE_ATTRIBUTE_OPTIONS_STOP_ON_ERROR: i32 = 0x01;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attr: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyMultipleAttributeValues(
        element: AXUIElementRef,
        attrs: CFArrayRef,
        options: i32,
        values: *mut CFArrayRef,
    ) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attr: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> AXError;
    fn AXUIElementCopyActionNames(element: AXUIElementRef, names: *mut CFArrayRef) -> AXError;
    fn AXIsProcessTrusted() -> bool;
    fn AXValueGetTypeID() -> CFTypeID;
    fn AXValueGetValue(value: AXValueRef, value_type: i32, value_ptr: *mut c_void) -> bool;
    fn AXValueCreate(value_type: i32, value_ptr: *const c_void) -> AXValueRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const c_void;
    fn CFStringGetTypeID() -> CFTypeID;
    fn CFArrayGetTypeID() -> CFTypeID;
    fn CFNullGetTypeID() -> CFTypeID;
}

// ============================================================================
// MARK: - AX Attribute Constants (as lazy CFStrings)
// ============================================================================

macro_rules! ax_attr {
    ($name:ident, $val:expr) => {
        fn $name() -> CFString {
            CFString::new($val)
        }
    };
}

ax_attr!(k_ax_role, "AXRole");
ax_attr!(k_ax_title, "AXTitle");
ax_attr!(k_ax_description, "AXDescription");
ax_attr!(k_ax_value, "AXValue");
ax_attr!(k_ax_children, "AXChildren");
ax_attr!(k_ax_windows, "AXWindows");
ax_attr!(k_ax_position, "AXPosition");
ax_attr!(k_ax_size, "AXSize");
ax_attr!(k_ax_focused_application, "AXFocusedApplication");
ax_attr!(k_ax_selected_text_range, "AXSelectedTextRange");
ax_attr!(k_ax_selected_text, "AXSelectedText");

fn k_ax_press_action() -> CFString {
    CFString::new("AXPress")
}

// ============================================================================
// MARK: - Safe AX Attribute Access Helpers
// ============================================================================

/// Safely get a string attribute from an AX element.
fn safe_get_string(element: AXUIElementRef, attr: &CFString) -> Option<String> {
    safe_get_string_profiled(element, attr, None)
}

/// Safely get a string attribute, optionally counting the query.
fn safe_get_string_profiled(
    element: AXUIElementRef,
    attr: &CFString,
    profile: Option<&mut SnapshotProfile>,
) -> Option<String> {
    if let Some(p) = profile {
        p.count_queries(1);
    }
    unsafe {
        let mut value: CFTypeRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            element,
            attr.as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            return None;
        }

        let type_id = CFGetTypeID(value);
        if type_id == CFStringGetTypeID() {
            let cf_str: CFString = TCFType::wrap_under_create_rule(value as CFStringRef);
            Some(cf_str.to_string())
        } else {
            CFRelease(value);
            None
        }
    }
}

/// Safely get the frame (position + size) of an AX element, with optional profiling.
fn safe_get_frame_profiled(element: AXUIElementRef, profile: Option<&mut SnapshotProfile>) -> Option<Rect> {
    if let Some(p) = profile {
        p.count_queries(2); // position + size
    }
    safe_get_frame_inner(element)
}

/// Safely get the frame (position + size) of an AX element.
fn safe_get_frame(element: AXUIElementRef) -> Option<Rect> {
    safe_get_frame_inner(element)
}

fn safe_get_frame_inner(element: AXUIElementRef) -> Option<Rect> {
    unsafe {
        let mut pos_value: CFTypeRef = std::ptr::null_mut();
        let mut size_value: CFTypeRef = std::ptr::null_mut();

        let pos_err = AXUIElementCopyAttributeValue(
            element,
            k_ax_position().as_concrete_TypeRef(),
            &mut pos_value,
        );
        let size_err = AXUIElementCopyAttributeValue(
            element,
            k_ax_size().as_concrete_TypeRef(),
            &mut size_value,
        );

        if pos_err != K_AX_ERROR_SUCCESS
            || size_err != K_AX_ERROR_SUCCESS
            || pos_value.is_null()
            || size_value.is_null()
        {
            if !pos_value.is_null() {
                CFRelease(pos_value);
            }
            if !size_value.is_null() {
                CFRelease(size_value);
            }
            return None;
        }

        // Verify they are AXValue types
        let ax_val_type_id = AXValueGetTypeID();
        if CFGetTypeID(pos_value) != ax_val_type_id || CFGetTypeID(size_value) != ax_val_type_id {
            CFRelease(pos_value);
            CFRelease(size_value);
            return None;
        }

        #[repr(C)]
        struct CGPoint {
            x: f64,
            y: f64,
        }
        #[repr(C)]
        struct CGSize {
            width: f64,
            height: f64,
        }

        let mut point = CGPoint { x: 0.0, y: 0.0 };
        let mut size = CGSize {
            width: 0.0,
            height: 0.0,
        };

        let got_point = AXValueGetValue(
            pos_value as AXValueRef,
            K_AX_VALUE_TYPE_CG_POINT,
            &mut point as *mut _ as *mut c_void,
        );
        let got_size = AXValueGetValue(
            size_value as AXValueRef,
            K_AX_VALUE_TYPE_CG_SIZE,
            &mut size as *mut _ as *mut c_void,
        );

        CFRelease(pos_value);
        CFRelease(size_value);

        if got_point && got_size {
            Some(Rect {
                x: point.x,
                y: point.y,
                width: size.width,
                height: size.height,
            })
        } else {
            None
        }
    }
}

/// Get action names, with optional profiling.
fn safe_get_actions_profiled(element: AXUIElementRef, profile: Option<&mut SnapshotProfile>) -> Vec<String> {
    if let Some(p) = profile {
        p.count_queries(1);
    }
    safe_get_actions(element)
}

/// Get action names for an AX element.
fn safe_get_actions(element: AXUIElementRef) -> Vec<String> {
    unsafe {
        let mut names: CFArrayRef = std::ptr::null_mut();
        let err = AXUIElementCopyActionNames(element, &mut names);
        if err != K_AX_ERROR_SUCCESS || names.is_null() {
            return Vec::new();
        }

        let count = CFArrayGetCount(names);
        let mut result = Vec::with_capacity(count as usize);
        for i in 0..count {
            let val = CFArrayGetValueAtIndex(names, i);
            if !val.is_null() && CFGetTypeID(val as CFTypeRef) == CFStringGetTypeID() {
                let cf_str: CFString =
                    TCFType::wrap_under_get_rule(val as CFStringRef);
                result.push(cf_str.to_string());
            }
        }
        CFRelease(names as CFTypeRef);
        result
    }
}

/// Get children, with optional profiling.
fn safe_get_children_profiled(element: AXUIElementRef, profile: Option<&mut SnapshotProfile>) -> Vec<AXUIElementRef> {
    if let Some(p) = profile {
        p.count_queries(1);
    }
    safe_get_children(element)
}

/// Get children of an AX element (returns raw AXUIElementRefs).
fn safe_get_children(element: AXUIElementRef) -> Vec<AXUIElementRef> {
    unsafe {
        let mut value: CFTypeRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            element,
            k_ax_children().as_concrete_TypeRef(),
            &mut value,
        );
        if err != K_AX_ERROR_SUCCESS || value.is_null() {
            return Vec::new();
        }

        let type_id = CFGetTypeID(value);
        if type_id != CFArrayGetTypeID() {
            CFRelease(value);
            return Vec::new();
        }

        let array = value as CFArrayRef;
        let count = CFArrayGetCount(array);
        let mut result = Vec::with_capacity(count as usize);
        for i in 0..count {
            let child = CFArrayGetValueAtIndex(array, i) as AXUIElementRef;
            if !child.is_null() {
                CFRetain(child as CFTypeRef);
                result.push(child);
            }
        }
        CFRelease(value);
        result
    }
}

/// Batch fetch with optional profiling.
fn batch_get_attributes_profiled(
    element: AXUIElementRef,
    profile: Option<&mut SnapshotProfile>,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    if let Some(p) = profile {
        p.count_queries(4); // role, title, description, value
    }
    batch_get_attributes(element)
}

/// Batch fetch role, title, description, value using AXUIElementCopyMultipleAttributeValues.
fn batch_get_attributes(
    element: AXUIElementRef,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    unsafe {
        let attrs: CFArray<CFString> = CFArray::from_CFTypes(&[
            k_ax_role(),
            k_ax_title(),
            k_ax_description(),
            k_ax_value(),
        ]);

        let mut values: CFArrayRef = std::ptr::null_mut();
        let err = AXUIElementCopyMultipleAttributeValues(
            element,
            attrs.as_concrete_TypeRef(),
            K_AX_COPY_MULTIPLE_ATTRIBUTE_OPTIONS_STOP_ON_ERROR,
            &mut values,
        );

        if (err == K_AX_ERROR_SUCCESS || err == K_AX_ERROR_ATTRIBUTE_UNSUPPORTED) && !values.is_null()
        {
            let count = CFArrayGetCount(values);
            let extract_str = |idx: isize| -> Option<String> {
                if idx >= count {
                    return None;
                }
                let val = CFArrayGetValueAtIndex(values, idx);
                if val.is_null() {
                    return None;
                }
                let type_id = CFGetTypeID(val as CFTypeRef);
                if type_id == CFNullGetTypeID() {
                    return None;
                }
                if type_id == CFStringGetTypeID() {
                    let cf_str: CFString =
                        TCFType::wrap_under_get_rule(val as CFStringRef);
                    Some(cf_str.to_string())
                } else {
                    None
                }
            };

            let result = (extract_str(0), extract_str(1), extract_str(2), extract_str(3));
            CFRelease(values as CFTypeRef);
            return result;
        }

        if !values.is_null() {
            CFRelease(values as CFTypeRef);
        }

        // Fallback to individual fetches
        (
            safe_get_string(element, &k_ax_role()),
            safe_get_string(element, &k_ax_title()),
            safe_get_string(element, &k_ax_description()),
            safe_get_string(element, &k_ax_value()),
        )
    }
}

// ============================================================================
// MARK: - AX Tree Node
// ============================================================================

/// A node in the accessibility tree.
pub struct AXNode {
    pub role: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub frame: Option<Rect>,
    pub actions: Vec<String>,
    pub is_interactive: bool,
    pub children: Vec<AXNode>,
    pub depth: usize,
    pub path_segment: PathSegment,
}

// ============================================================================
// MARK: - AX Tree Traversal
// ============================================================================

/// Recursively traverse the AX tree with optional profiling instrumentation.
fn traverse_ax_tree_profiled(
    element: AXUIElementRef,
    depth: usize,
    max_depth: usize,
    deadline: Instant,
    index_in_parent: usize,
    mut profile: Option<&mut SnapshotProfile>,
) -> Option<AXNode> {
    // Check timeout
    if Instant::now() > deadline {
        return None;
    }

    // Check depth
    if depth > max_depth {
        return None;
    }

    let node_start = Instant::now();

    let (role_opt, title, description, value) = if let Some(ref mut p) = profile.as_deref_mut() {
        batch_get_attributes_profiled(element, Some(*p))
    } else {
        batch_get_attributes(element)
    };
    let role = role_opt.unwrap_or_else(|| "AXUnknown".to_string());
    let frame = if let Some(ref mut p) = profile.as_deref_mut() {
        safe_get_frame_profiled(element, Some(*p))
    } else {
        safe_get_frame(element)
    };
    let actions = if let Some(ref mut p) = profile.as_deref_mut() {
        safe_get_actions_profiled(element, Some(*p))
    } else {
        safe_get_actions(element)
    };
    let is_interactive = INTERACTIVE_ROLES.contains(role.as_str());

    let segment = PathSegment {
        role: role.clone(),
        index: index_in_parent,
    };

    // Get children and recurse
    let child_elements = if let Some(ref mut p) = profile.as_deref_mut() {
        safe_get_children_profiled(element, Some(*p))
    } else {
        safe_get_children(element)
    };
    let mut child_nodes: Vec<AXNode> = Vec::new();
    let mut child_role_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Record this node's own time (before recursing into children)
    let self_duration = node_start.elapsed();
    if let Some(ref mut p) = profile.as_deref_mut() {
        p.record(depth, self_duration);
    }

    for child in &child_elements {
        // Get child role for path indexing
        let child_role = if let Some(ref mut p) = profile.as_deref_mut() {
            safe_get_string_profiled(*child, &k_ax_role(), Some(*p))
                .unwrap_or_else(|| "AXUnknown".to_string())
        } else {
            safe_get_string(*child, &k_ax_role()).unwrap_or_else(|| "AXUnknown".to_string())
        };
        let child_idx = child_role_counts.entry(child_role).or_insert(0);
        let current_idx = *child_idx;
        *child_idx += 1;

        if let Some(child_node) = traverse_ax_tree_profiled(
            *child,
            depth + 1,
            max_depth,
            deadline,
            current_idx,
            profile.as_deref_mut(),
        ) {
            child_nodes.push(child_node);
        }
    }

    // Release child element refs
    unsafe {
        for child in &child_elements {
            CFRelease(*child as CFTypeRef);
        }
    }

    Some(AXNode {
        role,
        title,
        description,
        value: if is_interactive { value } else { None },
        frame,
        actions,
        is_interactive,
        children: child_nodes,
        depth,
        path_segment: segment,
    })
}

// ============================================================================
// MARK: - Snapshot for PID
// ============================================================================

/// Take a snapshot of an application's accessibility tree.
/// Returns (tree_nodes, app_name, window_title).
pub fn take_snapshot(
    pid: i32,
    depth: u32,
    timeout_secs: f64,
) -> (Vec<AXNode>, String, Option<String>) {
    let (nodes, name, title, _profile) = take_snapshot_profiled(pid, depth, timeout_secs);
    (nodes, name, title)
}

/// Take a snapshot with profiling instrumentation.
/// Returns (tree_nodes, app_name, window_title, profile).
pub fn take_snapshot_profiled(
    pid: i32,
    depth: u32,
    timeout_secs: f64,
) -> (Vec<AXNode>, String, Option<String>, SnapshotProfile) {
    let snapshot_start = Instant::now();
    let mut profile = SnapshotProfile::new();

    let app_element = unsafe { AXUIElementCreateApplication(pid) };
    let deadline = Instant::now() + std::time::Duration::from_secs_f64(timeout_secs);

    // Get app name (count as 1 AX query)
    profile.count_queries(1);
    let app_name = safe_get_string(app_element, &k_ax_title()).unwrap_or_else(|| "Unknown".to_string());

    // Get windows
    let mut window_title: Option<String> = None;
    let mut root_nodes: Vec<AXNode> = Vec::new();

    unsafe {
        let mut windows_value: CFTypeRef = std::ptr::null_mut();
        profile.count_queries(1); // kAXWindows query
        let err = AXUIElementCopyAttributeValue(
            app_element,
            k_ax_windows().as_concrete_TypeRef(),
            &mut windows_value,
        );

        if err == K_AX_ERROR_SUCCESS && !windows_value.is_null() {
            let type_id = CFGetTypeID(windows_value);
            if type_id == CFArrayGetTypeID() {
                let array = windows_value as CFArrayRef;
                let count = CFArrayGetCount(array);

                for win_idx in 0..count {
                    let window = CFArrayGetValueAtIndex(array, win_idx) as AXUIElementRef;
                    if window.is_null() {
                        continue;
                    }

                    profile.count_queries(1); // window title query
                    let win_title = safe_get_string(window, &k_ax_title());
                    if win_idx == 0 {
                        window_title = win_title;
                    }

                    if let Some(node) = traverse_ax_tree_profiled(
                        window,
                        0,
                        depth as usize,
                        deadline,
                        win_idx as usize,
                        Some(&mut profile),
                    ) {
                        root_nodes.push(node);
                    }
                }
            }
            CFRelease(windows_value);
        } else {
            // No windows — traverse the app element itself
            if let Some(node) = traverse_ax_tree_profiled(
                app_element,
                0,
                depth as usize,
                deadline,
                0,
                Some(&mut profile),
            ) {
                root_nodes.push(node);
            }
        }

        CFRelease(app_element as CFTypeRef);
    }

    profile.total_duration = snapshot_start.elapsed();
    (root_nodes, app_name, window_title, profile)
}

// ============================================================================
// MARK: - Interactive Filtering + Snapshot Formatter (Task 4.2)
// ============================================================================

/// Check if a node has any interactive descendants.
fn node_has_interactive_descendants(node: &AXNode) -> bool {
    if node.is_interactive {
        return true;
    }
    for child in &node.children {
        if node_has_interactive_descendants(child) {
            return true;
        }
    }
    false
}

/// Structural roles that provide context (shown even in interactive-only mode).
const CONTEXT_ROLES: &[&str] = &[
    "AXWindow",
    "AXToolbar",
    "AXGroup",
    "AXScrollArea",
    "AXSplitGroup",
    "AXTabGroup",
    "AXSheet",
    "AXMenuBar",
    "AXList",
    "AXOutline",
    "AXTable",
    "AXBrowser",
    "AXWebArea",
    "AXApplication",
];

/// Format the snapshot tree as text and collect element refs.
/// Returns (formatted_text, element_refs).
pub fn format_snapshot_text(
    tree: &[AXNode],
    app_name: &str,
    window_title: &Option<String>,
    interactive_only: bool,
    pid: i32,
) -> (String, Vec<ElementRef>) {
    let mut output = String::new();
    let mut refs: Vec<ElementRef> = Vec::new();
    let mut ref_counter: u32 = 1;

    // Header
    if let Some(win_title) = window_title {
        if !win_title.is_empty() {
            output.push_str(&format!("[{app_name} — {win_title}]\n"));
        } else {
            output.push_str(&format!("[{app_name}]\n"));
        }
    } else {
        output.push_str(&format!("[{app_name}]\n"));
    }

    fn format_node(
        node: &AXNode,
        indent: usize,
        path: &[PathSegment],
        output: &mut String,
        refs: &mut Vec<ElementRef>,
        ref_counter: &mut u32,
        interactive_only: bool,
        pid: i32,
    ) {
        let mut current_path = path.to_vec();
        current_path.push(node.path_segment.clone());
        let indent_str: String = "  ".repeat(indent);

        if node.is_interactive {
            let ref_id = format!("e{}", *ref_counter);
            *ref_counter += 1;

            // Build label
            let label = node
                .title
                .as_deref()
                .or(node.description.as_deref())
                .or(node.value.as_deref());
            let mut line = format!("{indent_str}@{ref_id} {}", node.role);
            if let Some(l) = label {
                if !l.is_empty() {
                    let truncated = if l.len() > 60 {
                        format!("{}...", &l[..57.min(l.len())])
                    } else {
                        l.to_string()
                    };
                    line.push_str(&format!(" \"{truncated}\""));
                }
            }
            output.push_str(&line);
            output.push('\n');

            // Store ref
            refs.push(ElementRef {
                id: ref_id,
                source: RefSource::AX,
                role: node.role.clone(),
                label: label.map(|s| s.to_string()),
                frame: node.frame.clone(),
                ax_path: Some(current_path.clone()),
                ax_actions: Some(node.actions.clone()),
                ax_pid: Some(pid),
                cdp_node_id: None,
                cdp_backend_node_id: None,
                cdp_port: None,
                ab_ref: None,
                ab_session: None,
            });
        } else if !interactive_only {
            // Show structural parents for context
            let has_interactive = node_has_interactive_descendants(node);
            if CONTEXT_ROLES.contains(&node.role.as_str()) && has_interactive {
                let label = node.title.as_deref().or(node.description.as_deref());
                let mut line = format!("{indent_str}{}", node.role);
                if let Some(l) = label {
                    if !l.is_empty() {
                        let truncated = if l.len() > 40 {
                            format!("{}...", &l[..37.min(l.len())])
                        } else {
                            l.to_string()
                        };
                        line.push_str(&format!(" \"{truncated}\""));
                    }
                }
                output.push_str(&line);
                output.push('\n');
            }
        }

        // Recurse children
        let next_indent = if node.is_interactive || !interactive_only {
            indent + 1
        } else {
            indent
        };
        for child in &node.children {
            format_node(
                child,
                next_indent,
                &current_path,
                output,
                refs,
                ref_counter,
                interactive_only,
                pid,
            );
        }
    }

    for node in tree {
        format_node(
            node,
            1,
            &[],
            &mut output,
            &mut refs,
            &mut ref_counter,
            interactive_only,
            pid,
        );
    }

    (output, refs)
}

// ============================================================================
// MARK: - AX-first Headless Actions (Task 4.3)
// ============================================================================

/// Perform AXPress action on an element (headless, works on background apps).
pub fn ax_press(element: AXUIElementRef) -> Result<(), String> {
    unsafe {
        let action = k_ax_press_action();
        let err = AXUIElementPerformAction(element, action.as_concrete_TypeRef());
        if err == K_AX_ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("AXPress failed with error code {err}"))
        }
    }
}

/// Set value on an element via kAXValueAttribute, with read-back verification.
pub fn ax_set_value(element: AXUIElementRef, text: &str) -> Result<(), String> {
    unsafe {
        let attr = k_ax_value();
        let cf_text = CFString::new(text);

        let err = AXUIElementSetAttributeValue(
            element,
            attr.as_concrete_TypeRef(),
            cf_text.as_CFTypeRef(),
        );

        if err != K_AX_ERROR_SUCCESS {
            return Err(format!("AXSetValue failed with error code {err}"));
        }

        // Read-back verification
        let read_back = safe_get_string(element, &k_ax_value());
        if let Some(val) = read_back {
            if val == text {
                return Ok(());
            }
        }

        // Value was set but read-back doesn't match — fall through to selection replace
        Err("AXSetValue returned success but read-back verification failed".to_string())
    }
}

/// Replace text using selection-based approach:
/// 1. Select all text via kAXSelectedTextRangeAttribute
/// 2. Replace with kAXSelectedTextAttribute
pub fn ax_selection_replace(element: AXUIElementRef, text: &str) -> Result<(), String> {
    unsafe {
        // First, get current text length to select all
        let current_len = safe_get_string(element, &k_ax_value())
            .map(|s| s.len() as i64)
            .unwrap_or(100_000); // large number to select everything

        // Create CFRange covering all text
        #[repr(C)]
        struct CFRange {
            location: i64,
            length: i64,
        }
        let range = CFRange {
            location: 0,
            length: current_len,
        };

        let ax_range = AXValueCreate(
            K_AX_VALUE_TYPE_CF_RANGE,
            &range as *const _ as *const c_void,
        );
        if ax_range.is_null() {
            return Err("Failed to create AXValue for range".to_string());
        }

        // Set selection range
        let sel_range_attr = k_ax_selected_text_range();
        let err = AXUIElementSetAttributeValue(
            element,
            sel_range_attr.as_concrete_TypeRef(),
            ax_range as CFTypeRef,
        );
        CFRelease(ax_range as CFTypeRef);

        if err != K_AX_ERROR_SUCCESS {
            // Fallback: Cmd+A via CGEvent to select all
            crate::input::key_press(0, core_graphics::event::CGEventFlags::CGEventFlagCommand);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Replace selected text
        let sel_text_attr = k_ax_selected_text();
        let cf_text = CFString::new(text);
        let err = AXUIElementSetAttributeValue(
            element,
            sel_text_attr.as_concrete_TypeRef(),
            cf_text.as_CFTypeRef(),
        );

        if err == K_AX_ERROR_SUCCESS {
            Ok(())
        } else {
            // Final fallback: type the text via keyboard
            crate::input::type_string(text);
            Ok(())
        }
    }
}

// ============================================================================
// MARK: - Frontmost App Detection (Task 4.4)
// ============================================================================

/// Detect the frontmost application using 3-tier fallback:
/// 1. AX systemWide → kAXFocusedApplicationAttribute
/// 2. NSWorkspace.frontmostApplication (via objc2)
/// 3. CGWindowListCopyWindowInfo (first window at layer 0)
pub fn get_frontmost_app() -> Option<(String, i32)> {
    // Tier 1: AX systemWide
    if let Some(result) = get_frontmost_via_ax() {
        return Some(result);
    }

    // Tier 2: NSWorkspace
    if let Some(result) = get_frontmost_via_nsworkspace() {
        return Some(result);
    }

    // Tier 3: CGWindowList
    get_frontmost_via_cgwindowlist()
}

/// Tier 1: Use AX system-wide element to get focused application.
fn get_frontmost_via_ax() -> Option<(String, i32)> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        let mut focused_app: CFTypeRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            k_ax_focused_application().as_concrete_TypeRef(),
            &mut focused_app,
        );

        if err != K_AX_ERROR_SUCCESS || focused_app.is_null() {
            CFRelease(system_wide as CFTypeRef);
            return None;
        }

        let app_element = focused_app as AXUIElementRef;
        let mut pid: i32 = 0;
        AXUIElementGetPid(app_element, &mut pid);
        let name =
            safe_get_string(app_element, &k_ax_title()).unwrap_or_else(|| "Unknown".to_string());

        CFRelease(focused_app);
        CFRelease(system_wide as CFTypeRef);
        Some((name, pid))
    }
}

/// Tier 2: Use NSWorkspace.frontmostApplication via objc2-app-kit.
fn get_frontmost_via_nsworkspace() -> Option<(String, i32)> {
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication();
    let app = app?;

    let pid = app.processIdentifier();
    let name = app
        .localizedName()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Some((name, pid))
}

/// Tier 3: Use CGWindowListCopyWindowInfo to find the frontmost window at layer 0.
fn get_frontmost_via_cgwindowlist() -> Option<(String, i32)> {
    use core_foundation::dictionary::CFDictionaryRef;
    use core_foundation::number::CFNumber;
    use core_graphics::window::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
    };

    let info = core_graphics::window::copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )?;

    let my_pid = std::process::id() as i32;
    let count = info.len();

    // Iterate windows in front-to-back order
    for i in 0..count {
        unsafe {
            let dict_ref = CFArrayGetValueAtIndex(
                info.as_concrete_TypeRef(),
                i as isize,
            ) as CFDictionaryRef;
            if dict_ref.is_null() {
                continue;
            }

            // Get window layer
            let layer_key = CFString::new("kCGWindowLayer");
            let mut layer_value: *const c_void = std::ptr::null();
            if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict_ref,
                layer_key.as_CFTypeRef() as *const c_void,
                &mut layer_value,
            ) == 0
            {
                continue;
            }
            if layer_value.is_null() {
                continue;
            }
            let layer_num: CFNumber = TCFType::wrap_under_get_rule(layer_value as *const _);
            let layer: i32 = layer_num.to_i32().unwrap_or(-1);
            if layer != 0 {
                continue;
            }

            // Get PID
            let pid_key = CFString::new("kCGWindowOwnerPID");
            let mut pid_value: *const c_void = std::ptr::null();
            if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict_ref,
                pid_key.as_CFTypeRef() as *const c_void,
                &mut pid_value,
            ) == 0
            {
                continue;
            }
            if pid_value.is_null() {
                continue;
            }
            let pid_num: CFNumber = TCFType::wrap_under_get_rule(pid_value as *const _);
            let pid: i32 = pid_num.to_i32().unwrap_or(0);
            if pid == my_pid || pid == 0 {
                continue;
            }

            // Get owner name
            let name_key = CFString::new("kCGWindowOwnerName");
            let mut name_value: *const c_void = std::ptr::null();
            let name = if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict_ref,
                name_key.as_CFTypeRef() as *const c_void,
                &mut name_value,
            ) != 0
                && !name_value.is_null()
            {
                let cf_str: CFString = TCFType::wrap_under_get_rule(name_value as CFStringRef);
                cf_str.to_string()
            } else {
                "Unknown".to_string()
            };

            return Some((name, pid));
        }
    }

    None
}

/// Check if accessibility is trusted (permission granted).
pub fn is_process_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Get the focused window title for a given PID using AX APIs.
/// Uses AXUIElement → kAXFocusedWindowAttribute → kAXTitleAttribute.
pub fn get_window_title_for_pid(pid: i32) -> Option<String> {
    unsafe {
        let app_element = AXUIElementCreateApplication(pid);

        // Get the focused window attribute
        let focused_window_attr = CFString::new("AXFocusedWindow");
        let mut window_ref: CFTypeRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            app_element,
            focused_window_attr.as_concrete_TypeRef(),
            &mut window_ref,
        );

        if err != K_AX_ERROR_SUCCESS || window_ref.is_null() {
            CFRelease(app_element as CFTypeRef);
            return None;
        }

        // Get the title of the focused window
        let title = safe_get_string(window_ref as AXUIElementRef, &k_ax_title());

        CFRelease(window_ref);
        CFRelease(app_element as CFTypeRef);

        title.filter(|t| !t.is_empty())
    }
}

// ============================================================================
// MARK: - Path Re-traversal (for resolving stored refs back to live elements)
// ============================================================================

/// Re-traverse from a stored path to get the live AXUIElement.
/// Returns a retained AXUIElementRef that the caller must CFRelease.
pub fn re_traverse_to_element(path: &[PathSegment], pid: i32) -> Option<AXUIElementRef> {
    if path.is_empty() {
        return None;
    }

    unsafe {
        let app_element = AXUIElementCreateApplication(pid);

        // First segment is the window
        let mut windows_value: CFTypeRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            app_element,
            k_ax_windows().as_concrete_TypeRef(),
            &mut windows_value,
        );

        if err != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            CFRelease(app_element as CFTypeRef);
            return None;
        }

        let array = windows_value as CFArrayRef;
        let count = CFArrayGetCount(array);

        let first_seg = &path[0];
        if first_seg.index as isize >= count {
            CFRelease(windows_value);
            CFRelease(app_element as CFTypeRef);
            return None;
        }

        let mut current =
            CFArrayGetValueAtIndex(array, first_seg.index as isize) as AXUIElementRef;
        CFRetain(current as CFTypeRef);

        // Walk remaining path segments
        for seg in &path[1..] {
            let children = safe_get_children(current);

            let mut found = false;
            let mut role_count = 0usize;
            for child in &children {
                let child_role = safe_get_string(*child, &k_ax_role())
                    .unwrap_or_else(|| "AXUnknown".to_string());
                if child_role == seg.role {
                    if role_count == seg.index {
                        CFRelease(current as CFTypeRef);
                        current = *child;
                        CFRetain(current as CFTypeRef);
                        found = true;
                        // Release all children
                        for c in &children {
                            CFRelease(*c as CFTypeRef);
                        }
                        break;
                    }
                    role_count += 1;
                }
            }

            if !found {
                // Release remaining children
                for c in &children {
                    CFRelease(*c as CFTypeRef);
                }
                CFRelease(current as CFTypeRef);
                CFRelease(windows_value);
                CFRelease(app_element as CFTypeRef);
                return None;
            }
        }

        CFRelease(windows_value);
        CFRelease(app_element as CFTypeRef);
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_trusted() {
        // Smoke test — verifies the function doesn't crash
        let _trusted = is_process_trusted();
    }

    #[test]
    fn test_format_snapshot_basic() {
        let node = AXNode {
            role: "AXButton".to_string(),
            title: Some("OK".to_string()),
            description: None,
            value: None,
            frame: Some(Rect {
                x: 100.0,
                y: 200.0,
                width: 80.0,
                height: 30.0,
            }),
            actions: vec!["AXPress".to_string()],
            is_interactive: true,
            children: vec![],
            depth: 0,
            path_segment: PathSegment {
                role: "AXButton".to_string(),
                index: 0,
            },
        };

        let (text, refs) = format_snapshot_text(
            &[node],
            "TestApp",
            &Some("Main Window".to_string()),
            true,
            1234,
        );

        assert!(text.contains("[TestApp — Main Window]"));
        assert!(text.contains("@e1 AXButton \"OK\""));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].id, "e1");
        assert_eq!(refs[0].role, "AXButton");
        assert_eq!(refs[0].label, Some("OK".to_string()));
        assert_eq!(refs[0].ax_pid, Some(1234));
    }

    #[test]
    fn test_format_snapshot_label_truncation() {
        let long_label = "A".repeat(80);
        let node = AXNode {
            role: "AXTextField".to_string(),
            title: Some(long_label),
            description: None,
            value: None,
            frame: None,
            actions: vec![],
            is_interactive: true,
            children: vec![],
            depth: 0,
            path_segment: PathSegment {
                role: "AXTextField".to_string(),
                index: 0,
            },
        };

        let (text, _) = format_snapshot_text(&[node], "App", &None, true, 1);
        // Should be truncated to 57 chars + "..."
        assert!(text.contains("..."));
    }

    #[test]
    fn test_format_snapshot_context_roles() {
        let child = AXNode {
            role: "AXButton".to_string(),
            title: Some("Click".to_string()),
            description: None,
            value: None,
            frame: None,
            actions: vec![],
            is_interactive: true,
            children: vec![],
            depth: 1,
            path_segment: PathSegment {
                role: "AXButton".to_string(),
                index: 0,
            },
        };

        let parent = AXNode {
            role: "AXToolbar".to_string(),
            title: Some("Main Toolbar".to_string()),
            description: None,
            value: None,
            frame: None,
            actions: vec![],
            is_interactive: false,
            children: vec![child],
            depth: 0,
            path_segment: PathSegment {
                role: "AXToolbar".to_string(),
                index: 0,
            },
        };

        // interactive_only=false should show context roles
        let (text, refs) = format_snapshot_text(&[parent], "App", &None, false, 1);
        assert!(text.contains("AXToolbar \"Main Toolbar\""));
        assert!(text.contains("@e1 AXButton \"Click\""));
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn test_format_snapshot_non_interactive_skip() {
        let node = AXNode {
            role: "AXStaticText".to_string(),
            title: Some("Just text".to_string()),
            description: None,
            value: None,
            frame: None,
            actions: vec![],
            is_interactive: false,
            children: vec![],
            depth: 0,
            path_segment: PathSegment {
                role: "AXStaticText".to_string(),
                index: 0,
            },
        };

        let (text, refs) = format_snapshot_text(&[node], "App", &None, true, 1);
        assert!(!text.contains("AXStaticText"));
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_get_frontmost_app() {
        // Smoke test — should not crash. May return None in CI.
        let result = get_frontmost_app();
        if let Some((name, pid)) = result {
            assert!(!name.is_empty());
            assert!(pid > 0);
        }
    }
}
