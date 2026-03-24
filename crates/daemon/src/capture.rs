// Screenshot Engine — Screen capture using CGWindowListCreateImage (legacy but fast)
// and window frame data for coordinate translation.
//
// Note: We use the legacy CGWindowListCreateImage API because:
// - It's simpler to use from Rust (no async, no ScreenCaptureKit bindings needed)
// - It's faster (8.7ms vs 22.4ms average)
// - It still works on macOS 26 despite deprecation warnings
// - The screencapturekit-rs crate may not be stable enough
//
// We also use CGWindowListCopyWindowInfo for finding frontmost windows.

use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use core_graphics::image::CGImage;
use core_graphics::window::*;
use foreign_types::ForeignType as _ForeignType;
use std::ffi::c_void;

// Raw CFArray access
extern "C" {
    fn CFArrayGetValueAtIndex(array: core_foundation::array::CFArrayRef, idx: isize) -> *const c_void;
}

// Permission check FFI
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Check if screen recording permission is granted.
pub fn has_screen_recording_permission() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Request screen recording permission (shows system prompt if needed).
#[allow(dead_code)]
pub fn request_screen_recording_permission() -> bool {
    unsafe { CGRequestScreenCaptureAccess() }
}

/// Information about a captured window.
pub struct WindowInfo {
    pub window_id: u32,
    pub owner_name: String,
    pub owner_pid: i32,
    pub window_name: Option<String>,
    pub bounds: CGRect,
    pub layer: i32,
    pub is_on_screen: bool,
}

/// Result of a screenshot capture.
pub struct CaptureResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub scale: u32,
    pub window_origin_x: Option<f64>,
    pub window_origin_y: Option<f64>,
    pub app_name: Option<String>,
}

/// Get list of on-screen windows, sorted front-to-back.
pub fn get_window_list() -> Vec<WindowInfo> {
    let info = match copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    ) {
        Some(info) => info,
        None => return Vec::new(),
    };

    let mut windows = Vec::new();

    let count = info.len();
    for i in 0..count {
        unsafe {
            let dict_ref = CFArrayGetValueAtIndex(
                info.as_concrete_TypeRef(),
                i as isize,
            ) as CFDictionaryRef;
            if dict_ref.is_null() {
                continue;
            }

            let get_num = |key: &str| -> Option<i32> {
                let cf_key = CFString::new(key);
                let mut value: *const c_void = std::ptr::null();
                if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                    dict_ref,
                    cf_key.as_CFTypeRef() as *const c_void,
                    &mut value,
                ) != 0
                    && !value.is_null()
                {
                    let num: CFNumber = TCFType::wrap_under_get_rule(value as *const _);
                    num.to_i32()
                } else {
                    None
                }
            };

            let get_str = |key: &str| -> Option<String> {
                let cf_key = CFString::new(key);
                let mut value: *const c_void = std::ptr::null();
                if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                    dict_ref,
                    cf_key.as_CFTypeRef() as *const c_void,
                    &mut value,
                ) != 0
                    && !value.is_null()
                {
                    let cf_str: CFString = TCFType::wrap_under_get_rule(value as CFStringRef);
                    Some(cf_str.to_string())
                } else {
                    None
                }
            };

            let window_id = get_num("kCGWindowNumber").unwrap_or(0) as u32;
            let layer = get_num("kCGWindowLayer").unwrap_or(-1);
            let owner_pid = get_num("kCGWindowOwnerPID").unwrap_or(0);
            let owner_name = get_str("kCGWindowOwnerName").unwrap_or_else(|| "Unknown".to_string());
            let window_name = get_str("kCGWindowName");

            // Get bounds from dictionary
            let bounds = get_window_bounds(dict_ref).unwrap_or(CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(0.0, 0.0),
            ));

            let is_on_screen = get_num("kCGWindowIsOnscreen").unwrap_or(1) != 0;

            windows.push(WindowInfo {
                window_id,
                owner_name,
                owner_pid,
                window_name,
                bounds,
                layer,
                is_on_screen,
            });
        }
    }

    windows
}

/// Extract CGRect bounds from a window info dictionary.
unsafe fn get_window_bounds(dict_ref: CFDictionaryRef) -> Option<CGRect> {
    let bounds_key = CFString::new("kCGWindowBounds");
    let mut bounds_value: *const c_void = std::ptr::null();
    if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        dict_ref,
        bounds_key.as_CFTypeRef() as *const c_void,
        &mut bounds_value,
    ) == 0
        || bounds_value.is_null()
    {
        return None;
    }

    // kCGWindowBounds is a CFDictionary with X, Y, Width, Height
    let bounds_dict = bounds_value as CFDictionaryRef;

    let get_f64 = |key: &str| -> Option<f64> {
        let cf_key = CFString::new(key);
        let mut val: *const c_void = std::ptr::null();
        if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
            bounds_dict,
            cf_key.as_CFTypeRef() as *const c_void,
            &mut val,
        ) != 0
            && !val.is_null()
        {
            let num: CFNumber = TCFType::wrap_under_get_rule(val as *const _);
            num.to_f64()
        } else {
            None
        }
    };

    let x = get_f64("X")?;
    let y = get_f64("Y")?;
    let width = get_f64("Width")?;
    let height = get_f64("Height")?;

    Some(CGRect::new(
        &CGPoint::new(x, y),
        &CGSize::new(width, height),
    ))
}

/// Find the frontmost window, excluding our own process.
/// Returns (window_id, owner_name, owner_pid, bounds).
pub fn find_frontmost_window() -> Option<WindowInfo> {
    let my_pid = std::process::id() as i32;
    let windows = get_window_list();

    windows
        .into_iter()
        .find(|w| {
            w.owner_pid != my_pid
                && w.layer == 0
                && w.is_on_screen
                && w.bounds.size.width > 50.0
                && w.bounds.size.height > 50.0
        })
}

/// Find the window of a specific app by name.
pub fn find_app_window(app_name: &str) -> Option<WindowInfo> {
    let windows = get_window_list();
    let app_lower = app_name.to_lowercase();

    windows.into_iter().find(|w| {
        w.owner_name.to_lowercase().contains(&app_lower)
            && w.layer == 0
            && w.is_on_screen
            && w.bounds.size.width > 10.0
    })
}

/// Minimum valid PNG file size in bytes. Files smaller than this are likely blank.
const MIN_PNG_SIZE: u64 = 1024; // 1KB
/// Maximum retries for blank screenshot detection.
const BLANK_SCREENSHOT_MAX_RETRIES: u32 = 3;
/// Delay between retries in milliseconds.
const BLANK_SCREENSHOT_RETRY_DELAY_MS: u64 = 500;

/// Check if a PNG file is likely blank (too small to contain real content).
fn is_blank_screenshot(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.len() < MIN_PNG_SIZE,
        Err(_) => false, // If we can't read metadata, don't retry
    }
}

/// Capture a screenshot of a specific window or the frontmost window.
/// Saves as PNG to temp directory. Retries up to 3 times if result is blank (< 1KB).
pub fn capture_screenshot(full: bool, app: Option<&str>) -> Result<CaptureResult, String> {
    // Check permission
    if !has_screen_recording_permission() {
        return Err("Screen Recording permission required".to_string());
    }

    // Try capture with retry logic for blank screenshots (Task 6.1)
    let mut last_result: Option<CaptureResult> = None;

    for attempt in 0..=BLANK_SCREENSHOT_MAX_RETRIES {
        if attempt > 0 {
            eprintln!(
                "[capture] Screenshot appears blank (< {}B), retry {}/{}...",
                MIN_PNG_SIZE, attempt, BLANK_SCREENSHOT_MAX_RETRIES
            );
            std::thread::sleep(std::time::Duration::from_millis(BLANK_SCREENSHOT_RETRY_DELAY_MS));
        }

        let result = capture_screenshot_once(full, app)?;

        if !is_blank_screenshot(&result.path) {
            return Ok(result);
        }

        // Clean up blank screenshot file
        if attempt < BLANK_SCREENSHOT_MAX_RETRIES {
            let _ = std::fs::remove_file(&result.path);
        }
        last_result = Some(result);
    }

    // All retries exhausted — return error
    eprintln!(
        "[capture] WARNING: Screenshot still blank after {} retries",
        BLANK_SCREENSHOT_MAX_RETRIES
    );
    Err(format!(
        "Screenshot appears blank after {} retries (file < {}B). The window may not be fully rendered yet.",
        BLANK_SCREENSHOT_MAX_RETRIES, MIN_PNG_SIZE
    ))
}

/// Single-attempt screenshot capture (internal helper).
fn capture_screenshot_once(full: bool, app: Option<&str>) -> Result<CaptureResult, String> {
    if full {
        // Full screen capture
        let null_rect = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
        let image = create_image(
            null_rect, // CGRectNull = capture all screens
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )
        .ok_or("Failed to capture full screen")?;

        let path = save_image_as_png(&image)?;
        let width = image.width() as u32;
        let height = image.height() as u32;

        Ok(CaptureResult {
            path,
            width,
            height,
            scale: 1,
            window_origin_x: None,
            window_origin_y: None,
            app_name: None,
        })
    } else {
        // Window capture
        let window = if let Some(app_name) = app {
            find_app_window(app_name)
                .ok_or_else(|| format!("Window not found for app: {app_name}"))?
        } else {
            find_frontmost_window().ok_or("No frontmost window found")?
        };

        let image = create_image(
            CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0)), // CGRectNull
            kCGWindowListOptionIncludingWindow,
            window.window_id,
            kCGWindowImageBoundsIgnoreFraming | kCGWindowImageNominalResolution,
        )
        .ok_or("Failed to capture window")?;

        let path = save_image_as_png(&image)?;
        let width = image.width() as u32;
        let height = image.height() as u32;

        Ok(CaptureResult {
            path,
            width,
            height,
            scale: 1,
            window_origin_x: Some(window.bounds.origin.x),
            window_origin_y: Some(window.bounds.origin.y),
            app_name: Some(window.owner_name),
        })
    }
}

/// Save a CGImage as PNG to the temp directory.
fn save_image_as_png(image: &CGImage) -> Result<String, String> {
    // Create temp directory
    let temp_dir = std::env::temp_dir().join("agent-desktop");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("screenshot_{timestamp}.png");
    let file_path = temp_dir.join(&filename);

    // Use ImageIO to save as PNG
    unsafe {
        let url = core_foundation::url::CFURL::from_path(&file_path, false)
            .ok_or("Failed to create CFURL")?;

        let dest = CGImageDestinationCreateWithURL(
            url.as_concrete_TypeRef() as _,
            CFString::new("public.png").as_concrete_TypeRef(),
            1,
            std::ptr::null(),
        );
        if dest.is_null() {
            return Err("Failed to create image destination".to_string());
        }

        CGImageDestinationAddImage(dest, image.as_ptr() as *const c_void, std::ptr::null());
        let finalized = CGImageDestinationFinalize(dest);
        core_foundation::base::CFRelease(dest as CFTypeRef);

        if !finalized {
            return Err("Failed to finalize PNG".to_string());
        }
    }

    Ok(file_path.to_string_lossy().to_string())
}

// ImageIO FFI
#[link(name = "ImageIO", kind = "framework")]
extern "C" {
    fn CGImageDestinationCreateWithURL(
        url: *const c_void,
        r#type: CFStringRef,
        count: usize,
        options: *const c_void,
    ) -> *mut c_void;
    fn CGImageDestinationAddImage(
        dest: *mut c_void,
        image: *const c_void,  // CGImageRef
        properties: *const c_void,
    );
    fn CGImageDestinationFinalize(dest: *mut c_void) -> bool;
}

// ============================================================================
// MARK: - Coordinate Translation (Task 8.3)
// ============================================================================

/// Translate image-relative coordinates to screen coordinates.
/// screen_x = window_origin_x + image_x (at 1x/nominal resolution)
/// screen_y = window_origin_y + image_y
pub fn translate_image_to_screen(
    image_x: f64,
    image_y: f64,
    window_origin_x: f64,
    window_origin_y: f64,
) -> (f64, f64) {
    (window_origin_x + image_x, window_origin_y + image_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_translation() {
        let (sx, sy) = translate_image_to_screen(100.0, 200.0, 50.0, 75.0);
        assert_eq!(sx, 150.0);
        assert_eq!(sy, 275.0);
    }

    #[test]
    fn test_coordinate_translation_zero_origin() {
        let (sx, sy) = translate_image_to_screen(300.0, 400.0, 0.0, 0.0);
        assert_eq!(sx, 300.0);
        assert_eq!(sy, 400.0);
    }

    #[test]
    fn test_has_screen_recording_permission() {
        // This is a smoke test — just checks the function doesn't crash
        let _ = has_screen_recording_permission();
    }

    #[test]
    fn test_get_window_list() {
        // Smoke test — verifies we can call the window list API
        let windows = get_window_list();
        // On a running Mac, there should be at least some windows
        // (but in CI this might be empty)
        let _ = windows;
    }
}
