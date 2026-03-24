// Input Engine — CGEvent-based mouse, keyboard, typing, and scroll
//
// Key design decisions (from spike S3):
// - CGWarpMouseCursorPosition for mouse positioning (NOT .mouseMoved — drifts up to 288px)
// - keyboardSetUnicodeString for typing (handles ALL Unicode)
// - CGEvent keycodes for press commands (specific key behavior)
// - CGEvent scroll wheel for scroll

use core_graphics::display::CGDisplay;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, EventField, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::thread;
use std::time::Duration;

/// Create an event source for CGEvent creation.
/// Uses HIDSystemState which is appropriate for simulated input.
fn event_source() -> CGEventSource {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create CGEventSource")
}

/// Mouse click at absolute screen coordinates.
/// Uses CGWarpMouseCursorPosition for reliable absolute positioning,
/// then CGEvent for the actual click.
pub fn mouse_click(x: f64, y: f64, button: MouseButton, click_count: u32) {
    let point = CGPoint::new(x, y);

    // Warp cursor to position (reliable absolute positioning)
    let _ = CGDisplay::warp_mouse_cursor_position(point);
    thread::sleep(Duration::from_millis(10));

    let (down_type, up_type) = match button {
        MouseButton::Left => (CGEventType::LeftMouseDown, CGEventType::LeftMouseUp),
        MouseButton::Right => (CGEventType::RightMouseDown, CGEventType::RightMouseUp),
    };

    let cg_button = match button {
        MouseButton::Left => core_graphics::event::CGMouseButton::Left,
        MouseButton::Right => core_graphics::event::CGMouseButton::Right,
    };

    if click_count == 2 {
        // Double click: two click pairs with incrementing clickState
        let down1 = CGEvent::new_mouse_event(event_source(), down_type, point, cg_button);
        let up1 = CGEvent::new_mouse_event(event_source(), up_type, point, cg_button);
        let down2 = CGEvent::new_mouse_event(event_source(), down_type, point, cg_button);
        let up2 = CGEvent::new_mouse_event(event_source(), up_type, point, cg_button);

        if let (Ok(down1), Ok(up1), Ok(down2), Ok(up2)) = (down1, up1, down2, up2) {
            down1.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
            up1.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
            down2.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 2);
            up2.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 2);

            down1.post(CGEventTapLocation::HID);
            thread::sleep(Duration::from_millis(10));
            up1.post(CGEventTapLocation::HID);
            thread::sleep(Duration::from_millis(10));
            down2.post(CGEventTapLocation::HID);
            thread::sleep(Duration::from_millis(10));
            up2.post(CGEventTapLocation::HID);
        }
    } else {
        // Single click
        if let Ok(down) = CGEvent::new_mouse_event(event_source(), down_type, point, cg_button) {
            if let Ok(up) = CGEvent::new_mouse_event(event_source(), up_type, point, cg_button) {
                down.set_integer_value_field(
                    EventField::MOUSE_EVENT_CLICK_STATE,
                    click_count as i64,
                );
                up.set_integer_value_field(
                    EventField::MOUSE_EVENT_CLICK_STATE,
                    click_count as i64,
                );

                down.post(CGEventTapLocation::HID);
                thread::sleep(Duration::from_millis(10));
                up.post(CGEventTapLocation::HID);
            }
        }
    }

    thread::sleep(Duration::from_millis(30)); // settle after click
}

/// Key press with optional modifier flags.
/// Uses CGEvent keyboard events with virtual keycodes.
pub fn key_press(keycode: u16, modifiers: CGEventFlags) {
    if let Ok(key_down) = CGEvent::new_keyboard_event(event_source(), keycode, true) {
        if let Ok(key_up) = CGEvent::new_keyboard_event(event_source(), keycode, false) {
            key_down.set_flags(modifiers);
            key_up.set_flags(modifiers);

            key_down.post(CGEventTapLocation::HID);
            thread::sleep(Duration::from_millis(20));
            key_up.post(CGEventTapLocation::HID);
        }
    }
    thread::sleep(Duration::from_millis(10));
}

/// Type a string using keyboardSetUnicodeString.
/// Handles all Unicode characters. Chunks at 20 UTF-16 units.
/// Newlines are handled by pressing the Return key (keycode 36).
pub fn type_string(text: &str) {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let chunk_size = 20;
    let mut offset = 0;

    while offset < utf16.len() {
        // Find next chunk, splitting on newlines
        let mut end = std::cmp::min(offset + chunk_size, utf16.len());
        let mut has_newline = false;

        for i in offset..end {
            if utf16[i] == 0x000A {
                // newline
                end = i;
                has_newline = true;
                break;
            }
        }

        if has_newline && end == offset {
            // Current position is a newline — press Return
            key_press(36, CGEventFlags::CGEventFlagNull); // Return keycode
            offset += 1;
            continue;
        }

        if end > offset {
            let chunk = &utf16[offset..end];

            if let Ok(key_down) = CGEvent::new_keyboard_event(event_source(), 0, true) {
                if let Ok(key_up) = CGEvent::new_keyboard_event(event_source(), 0, false) {
                    key_down.set_string_from_utf16_unchecked(chunk);
                    key_up.set_string_from_utf16_unchecked(&[]);

                    key_down.post(CGEventTapLocation::HID);
                    thread::sleep(Duration::from_millis(20));
                    key_up.post(CGEventTapLocation::HID);
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }

        offset = end + if has_newline { 1 } else { 0 };
    }
}

/// Scroll in a direction with a given amount (in scroll lines).
pub fn scroll(direction: &str, amount: i32) {
    let (dy, dx) = match direction.to_lowercase().as_str() {
        "up" => (amount, 0),
        "down" => (-amount, 0),
        "left" => (0, amount),
        "right" => (0, -amount),
        _ => return,
    };

    if let Ok(scroll_event) = CGEvent::new_scroll_event(
        event_source(),
        ScrollEventUnit::LINE,
        2, // wheel_count: 2 (vertical + horizontal)
        dy,
        dx,
        0,
    ) {
        scroll_event.post(CGEventTapLocation::HID);
    }

    thread::sleep(Duration::from_millis(30));
}

/// Parse modifier strings into CGEventFlags.
pub fn parse_modifier_flags(modifiers: &[String]) -> CGEventFlags {
    let mut flags = CGEventFlags::CGEventFlagNull;
    for m in modifiers {
        match m.to_lowercase().as_str() {
            "cmd" | "command" | "meta" => flags |= CGEventFlags::CGEventFlagCommand,
            "shift" => flags |= CGEventFlags::CGEventFlagShift,
            "alt" | "option" | "opt" => flags |= CGEventFlags::CGEventFlagAlternate,
            "ctrl" | "control" => flags |= CGEventFlags::CGEventFlagControl,
            _ => {}
        }
    }
    flags
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
}
