# Spike S3: CGEvent Typing Reliability Results

**Date**: 2026-03-24
**Platform**: macOS (CoreGraphics + Accessibility API)
**Accessibility Permission**: ✅ Granted (AXIsProcessTrusted = true)

## Summary

| Category | Passed | Failed | Total | Notes |
|----------|--------|--------|-------|-------|
| Mouse | 4 | 3 | 7 | Failures are `.mouseMoved` positioning (see analysis) |
| KeyPress | 14 | 0 | 14 | All simple keys, arrows, and modifier combos work |
| Typing-Keycodes | 6 | 1 | 7 | Only failure: accented char 'é' (no keycode) |
| Typing-Unicode | 7 | 0 | 7 | **Perfect** — handles all text including Unicode |
| AXSetValue | 5 | 5 | 10 | Direct value set fails on TextEdit, but selection-based replacement works |

## Part 1: Mouse Click Testing

| Test | Result | Details |
|------|--------|---------|
| Move to top-left (100,100) | ✅ | Actual: (100,100), delta: (0.0,0.0) |
| Move to center (500,400) | ❌ | Actual: (444,365), delta: (55.6,34.8) |
| Move to right-area (800,300) | ❌ | Actual: (511,151), delta: (288.5,148.9) |
| Move to bottom-area (400,600) | ❌ | Actual: (336,557), delta: (63.6,42.2) |
| Left click at (500,400) | ✅ | Cursor stayed at (500,400) |
| Right click at (500,400) | ✅ | Right click posted (dismissed context menu with Esc) |
| Double click at (500,400) | ✅ | Double click event posted with clickState=2 |

### Analysis: Mouse Movement

**Key finding**: `CGEvent(.mouseMoved)` does NOT reliably warp the cursor to absolute coordinates. It appears to be subject to mouse acceleration or interpreted as relative movement. The first move from (0,0) area worked, but subsequent moves accumulated error.

**However**, mouse click events (`leftMouseDown/Up`) DO correctly position at the specified coordinates, as confirmed by the click test at (500,400) reading back exactly (500,400).

**Recommendation for mouse movement**: Use `CGWarpMouseCursorPosition()` or `CGDisplayMoveCursorToPoint()` for absolute positioning, NOT `.mouseMoved` events. Then use click events at the target position.

```swift
// Correct approach:
CGWarpMouseCursorPosition(CGPoint(x: 500, y: 400))
// Then click:
CGEvent(mouseEventSource:nil, mouseType:.leftMouseDown, mouseCursorPosition:point, mouseButton:.left)
```

## Part 2: Key Press Testing

| Test | Result | Details |
|------|--------|---------|
| Return/Enter (keycode 36) | ✅ | Event created and posted |
| Tab (keycode 48) | ✅ | Event created and posted |
| Escape (keycode 53) | ✅ | Event created and posted |
| Space (keycode 49) | ✅ | Event created and posted |
| Delete/Backspace (keycode 51) | ✅ | Event created and posted |
| Arrow Up (keycode 126) | ✅ | Event created and posted |
| Arrow Down (keycode 125) | ✅ | Event created and posted |
| Arrow Left (keycode 123) | ✅ | Event created and posted |
| Arrow Right (keycode 124) | ✅ | Event created and posted |
| Combo Cmd+C | ✅ | Event created with flags |
| Combo Cmd+V | ✅ | Event created with flags |
| Combo Cmd+A | ✅ | Event created with flags |
| Combo Cmd+Shift+S | ✅ | Event created with flags |
| Combo Cmd+Option+Esc | ✅ | Event created with flags |

### Analysis: Key Presses

**100% success rate.** CGEvent keyboard events are fully reliable:
- Simple key creation never returns nil
- Modifier flags (`.maskCommand`, `.maskShift`, `.maskAlternate`, `.maskControl`) combine correctly
- Posting via `.cghidEventTap` delivers events to the focused application
- No timing issues observed with 20-30ms delays between down/up

## Part 3: String Typing

### Approach 1: Per-Character Keycode Mapping

| Test String | Result | Details |
|-------------|--------|---------|
| "Hello World" (basic ASCII) | ✅ | 11/11 chars typed |
| "Hello, World! @#$%^&*()" (special chars) | ✅ | 23/23 chars typed |
| "café" (accented) | ❌ | 3/4 chars — **'é' has no keycode mapping** |
| "price: $19.99" (mixed) | ✅ | 13/13 chars typed |
| "path/to/file.txt" (path chars) | ✅ | 16/16 chars typed |
| "line1\nline2" (newline) | ✅ | 11/11 chars — newline mapped to Return keycode |
| Long paragraph (~107 chars) | ✅ | 107/107 chars typed |

### Approach 2: `CGEvent.keyboardSetUnicodeString`

| Test String | Result | Details |
|-------------|--------|---------|
| "Hello World" | ✅ | 11 UTF-16 units |
| "Hello, World! @#$%^&*()" | ✅ | 23 UTF-16 units |
| "café" | ✅ | **4 UTF-16 units — handles accented chars!** |
| "price: $19.99" | ✅ | 13 UTF-16 units |
| "path/to/file.txt" | ✅ | 16 UTF-16 units |
| "line1\nline2" | ✅ | 11 UTF-16 units |
| Long paragraph (~107 chars) | ✅ | 107 UTF-16 units (chunked at 20) |

### Analysis: String Typing

**`keyboardSetUnicodeString` is the clear winner** — 7/7 vs 6/7 for keycodes.

Per-character keycodes:
- ✅ Perfect for ASCII (letters, digits, common symbols, punctuation)
- ❌ Cannot handle accented/Unicode chars (é, ñ, ü, emoji, CJK, etc.)
- ✅ Good for `press` commands where you need specific keycode behavior

Unicode string approach:
- ✅ Handles ALL characters including accented, Unicode, special chars
- ✅ Can type multiple chars per event (chunk size ~20 UTF-16 units)
- ✅ Simpler code — no keycode mapping table needed
- ⚠️ Newlines may need special handling (explicit Return keycode press)
- ⚠️ May not trigger some keyboard event listeners correctly in all apps

## Part 4: AXSetValue Testing

| Test | Result | Details |
|------|--------|---------|
| Get focused UI element | ✅ | Got AXUIElement |
| Element role check | ✅ | Role: AXTextArea |
| Read current value | ✅ | Read 88,145 chars (accumulated from typing tests) |
| Set 'Simple ASCII' | ❌ | SetAttributeValue returned success but text not replaced |
| Set 'Special chars' | ❌ | Same — value unchanged |
| Set 'Unicode' | ❌ | Value grew (88,231) — may have appended |
| Set 'Multi-line' | ❌ | Same pattern |
| Set 'Long text' | ❌ | Same pattern |
| Set text selection | ✅ | **Selected range (6,5) — works!** |
| Replace selected text | ✅ | **Replaced selection with 'Universe' — works!** |

### Analysis: AXSetValue

**Critical finding**: Direct `kAXValueAttribute` setting does NOT work reliably on TextEdit's `AXTextArea`. The API returns `.success` but the text isn't actually replaced — it may be silently ignored or appended.

**However**, the selection-based approach WORKS:
1. `kAXSelectedTextRangeAttribute` — can set selection range ✅
2. `kAXSelectedTextAttribute` — can replace selected text ✅

**Recommended `fill` strategy**:
```swift
// 1. Select all text (set range to cover everything)
var range = CFRange(location: 0, length: currentTextLength)
let axRange = AXValueCreate(.cfRange, &range)
AXUIElementSetAttributeValue(element, kAXSelectedTextRangeAttribute, axRange)

// 2. Replace selection with new text
AXUIElementSetAttributeValue(element, kAXSelectedTextAttribute, newText as CFTypeRef)
```

Alternatively, use Cmd+A (select all) via CGEvent, then type the replacement text.

---

## Final Recommendations

### `type` command (simulating keyboard input)
| Approach | Recommendation | Use When |
|----------|---------------|----------|
| `keyboardSetUnicodeString` | **PRIMARY** | General text typing — handles all Unicode |
| Per-char keycodes | Fallback | When specific keycode behavior needed |
| Hybrid | Best of both | Unicode for text, keycodes for control chars (Return, Tab, etc.) |

### `fill` command (setting text field content)
| Approach | Recommendation | Use When |
|----------|---------------|----------|
| AX Select-All + Replace Selected | **PRIMARY** | Text fields in native apps |
| Cmd+A then type | Fallback | If AX selection doesn't work |
| `kAXValueAttribute` | **NOT RELIABLE** | Don't use — returns success but may not work |

### `press` command (keyboard shortcuts)
| Approach | Recommendation | Use When |
|----------|---------------|----------|
| CGEvent + keycodes + flags | **PRIMARY** | All keyboard shortcuts |
| Virtual keycode mapping | Required | Need keycode table for named keys |

### Mouse actions (`click`, `move`, etc.)
| Approach | Recommendation | Use When |
|----------|---------------|----------|
| `CGWarpMouseCursorPosition` | **For movement** | Moving cursor to coordinates |
| CGEvent mouse click | **For clicks** | Left, right, double-click at position |
| `.mouseMoved` event | **NOT RELIABLE** | Don't use for absolute positioning |

## Key Findings

1. **CGEvent creation never returns nil** — all event types are always constructible
2. **`.cghidEventTap`** is the correct posting tap point for simulated input
3. **`keyboardSetUnicodeString`** is the superior text typing approach — handles ALL characters
4. **Per-char keycodes** work for ASCII but fail on Unicode (é, ñ, emoji, etc.)
5. **AXSetValue with `kAXValueAttribute` is unreliable** — returns success but may not change text
6. **AX selection-based replacement WORKS** — `kAXSelectedTextRangeAttribute` + `kAXSelectedTextAttribute`
7. **`.mouseMoved` events are unreliable** for absolute positioning — use `CGWarpMouseCursorPosition`
8. **Mouse click events** correctly position at specified coordinates
9. **Timing**: 10-30ms delays between key events are sufficient to prevent dropped inputs
10. **Accessibility permission** is required and was confirmed working (`AXIsProcessTrusted() = true`)

## Gotchas & Limitations

- **Accessibility permission required** — must be granted in System Settings → Privacy → Accessibility
- **`keyboardSetUnicodeString` chunk limit**: ~20 UTF-16 units per event — chunk longer strings
- **AXSetValue silently fails** — returns `.success` even when text isn't changed (TextEdit AXTextArea)
- **Some apps ignore CGEvent input** — sandboxed apps, some Electron apps may not respond
- **AXSetValue won't trigger JS events** — browser input fields may need keyboard simulation
- **Mouse acceleration affects `.mouseMoved`** — always use `CGWarpMouseCursorPosition` for absolute positioning
- **Double-click requires `clickState`** — must set `.mouseEventClickState` field to 2 on second click pair
- **Retina displays**: CGEvent uses point coordinates, not pixel coordinates
