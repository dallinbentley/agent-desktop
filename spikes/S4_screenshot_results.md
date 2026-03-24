# Spike S4: ScreenCaptureKit Permission Flow — Results

**Date:** 2026-03-24  
**macOS:** 26.3.1 (Build 25D771280a)  
**Hardware:** Apple M3 Max, 3456×2234 Retina display (2x scaling → 1728×1117 logical)  

## 1. Permission Detection

### CGPreflightScreenCaptureAccess() — ✅ Works
- Returns `true` if Screen Recording permission is already granted
- Returns `false` if not granted — **no UI prompt, no side effects**
- Available since macOS 10.15.7, reliable on macOS 14+
- **This is the recommended pre-check** before attempting any capture

### CGRequestScreenCaptureAccess() — ✅ Works
- Returns `true` immediately if already granted
- Returns `false` and shows the System Preferences prompt if not granted
- The prompt directs users to Settings → Privacy & Security → Screen Recording
- **Must be called once** on first use; after that, preflight check suffices
- Note: On macOS 15+, there's a periodic re-authorization prompt from the system

### Permission Error Behavior
- **ScreenCaptureKit:** Throws an error (catchable) with a clear permission message
- **CGWindowListCreateImage:** Returns `nil` silently — harder to diagnose
- **Recommendation:** Always call `CGPreflightScreenCaptureAccess()` first, then guide the user

## 2. Capture Latency

### Single Capture Latency

| Method | Target | Latency | Dimensions | File Size |
|--------|--------|---------|------------|-----------|
| CGWindowListCreateImage | Full Screen | 40.9 ms | 3460×2234 | 1,121 KB |
| CGWindowListCreateImage | Window (Ghostty) | 8.1 ms | 3456×2168 | 915 KB |
| SCScreenshotManager | Full Screen (2x Retina) | 52.8 ms | 3456×2234 | 1,120 KB |
| SCScreenshotManager | Full Screen (1x Logical) | 24.2 ms | 1728×1117 | 992 KB |
| SCScreenshotManager | Window (Notification Center) | 20.4 ms | 360×360 | 29 KB |

### Benchmark: 10 Rapid Full-Screen Captures

| Method | Avg | Min | Max |
|--------|-----|-----|-----|
| SCScreenshotManager (1x) | **22.4 ms** | 21.3 ms | 23.6 ms |
| CGWindowListCreateImage | **8.7 ms** | 6.8 ms | 13.4 ms |

### SCShareableContent Fetch
- One-time content enumeration: **14.2 ms**
- Returns all displays, windows, and applications
- Can be cached and refreshed periodically

## 3. Retina Handling

| Config | Output Dimensions | Notes |
|--------|-------------------|-------|
| SCK width/height = display×2 | 3456×2234 (physical) | Full Retina quality, largest files |
| SCK width/height = display×1 | 1728×1117 (logical) | Half resolution, ~12% smaller files |
| CGWindowListCreateImage | 3460×2234 (physical) | Always captures at physical resolution |

**Key findings:**
- SCK gives full control over output resolution via `SCStreamConfiguration.width/height`
- Setting `captureResolution = .best` uses the best available resolution for the configured size
- Legacy API always returns physical (Retina) resolution — no control over output size
- The legacy API returns 3460 wide (slightly wider than 3456) due to including display bezels/edges

## 4. CGWindowListCreateImage (Legacy) Status

- **Deprecated in macOS 14.0** — compiler warns to use ScreenCaptureKit instead
- **Still functional on macOS 26.3** — returns valid images
- **Faster** than SCK for raw capture (8.7ms vs 22.4ms average)
- **Limitations:**
  - No resolution control (always physical)
  - Returns `nil` silently on permission failure
  - No async/await support
  - May be removed in future macOS versions
  - No metadata about captured content

## 5. File Sizes

| Capture | Size | Notes |
|---------|------|-------|
| Full screen 2x (3456×2234) | ~1,120 KB | Both methods produce similar sizes |
| Full screen 1x (1728×1117) | ~992 KB | Only ~12% smaller despite 4× fewer pixels |
| Single window 2x (360×360) | 30 KB | Small windows are very efficient |
| Single window (3456×2168 legacy) | 915 KB | Ghostty terminal window |

PNG compression makes the file size more dependent on content complexity than pixel count.

## 6. Recommendation for Implementation

### Use ScreenCaptureKit (SCScreenshotManager) as primary API

**Reasons:**
1. **Future-proof** — Legacy API is deprecated and will eventually be removed
2. **Resolution control** — Can capture at 1x or 2x depending on use case
3. **Window filtering** — Can capture specific windows without background
4. **Error handling** — Throws catchable errors instead of returning nil
5. **Content enumeration** — `SCShareableContent` provides rich metadata about available windows
6. **Cursor control** — Can include/exclude cursor via `showsCursor`

### Permission flow:
```swift
// 1. Pre-check (fast, no UI)
if CGPreflightScreenCaptureAccess() {
    // Proceed with capture
} else {
    // 2. Request (shows System Settings prompt)
    CGRequestScreenCaptureAccess()
    // 3. Guide user to grant permission
    // 4. Re-check after user action
}
```

### Latency strategy:
- **22ms average** is acceptable for AI agent use (agents don't need 60fps)
- For rapid sequential captures, consider reusing `SCContentFilter` objects
- Cache `SCShareableContent` and refresh only when window list changes
- Use 1x resolution for AI (sufficient for vision models, 24ms vs 53ms)

### Fallback: Keep CGWindowListCreateImage as backup
- Useful for environments where ScreenCaptureKit import fails
- Faster for simple full-screen grabs (8.7ms vs 22.4ms)
- Could be used as a degraded-mode fallback
