## MODIFIED Requirements

### Requirement: Key press inter-event timing
The delay between key-down and key-up events is reduced from 20ms to 2ms. The post-keystroke delay is reduced from 10ms to 1ms. Total per-keystroke overhead: 3ms (down from 30ms).

#### Scenario: Single key press
- **WHEN** `key_press(keycode, modifiers)` is called
- **THEN** key-down is posted, 2ms delay, key-up is posted, 1ms delay
- **THEN** total function time is under 5ms

#### Scenario: Rapid string typing
- **WHEN** `type_string("hello world")` is called
- **THEN** all characters are delivered correctly with no dropped keystrokes
- **THEN** total time for 11 characters is under 60ms (was ~330ms)

### Requirement: Mouse click timing
The existing mouse click delays (10ms between down/up) remain unchanged. Only keyboard timing is modified.

#### Scenario: Mouse click unchanged
- **WHEN** `mouse_click()` is called
- **THEN** timing behavior is identical to current implementation
