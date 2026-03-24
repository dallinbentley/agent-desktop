# Snapshot and Refs

Compact element references that reduce context usage dramatically for AI agents interacting with macOS apps.

**Related**: [commands.md](commands.md) for full command reference, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [How Refs Work](#how-refs-work)
- [The Snapshot Command](#the-snapshot-command)
- [Using Refs](#using-refs)
- [Ref Lifecycle](#ref-lifecycle)
- [Interactive Roles](#interactive-roles)
- [Ref Resolution](#ref-resolution)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## How Refs Work

Traditional screenshot approach:
```
Screenshot → Vision model parses → Guess coordinates → Action (~1500-3000 tokens, fragile)
```

agent-desktop approach:
```
Compact accessibility snapshot → @refs assigned → Direct interaction (~200-400 tokens, deterministic)
```

The snapshot system maps the macOS accessibility tree to a compact text representation. Each interactive element gets a sequential `@ref` handle (`@e1`, `@e2`, ...) that can be used in subsequent commands.

## The Snapshot Command

```bash
# Interactive snapshot — RECOMMENDED default
agent-desktop snapshot -i

# With options
agent-desktop snapshot -i -d 15      # Deeper traversal
agent-desktop snapshot -i -c         # Compact output
agent-desktop snapshot -i --app "Finder"  # Target specific app
```

### Snapshot Output Format

```
[TextEdit — Untitled.txt]
  toolbar:
    @e1 button "New"
    @e2 button "Open"
    @e3 button "Save"
  content:
    @e4 text_area "Document content area" (editable, 0 chars)
  menu_bar:
    @e5 menu "File"
    @e6 menu "Edit"
    @e7 menu "Format"
```

Key observations:
- **Header line**: `[AppName — WindowTitle]` identifies the target
- **Structure labels**: `toolbar:`, `content:`, `menu_bar:` provide spatial context (no refs — not interactive)
- **@ref lines**: `@e1 role "label" (attributes)` — these are actionable
- **Sequential numbering**: `@e1`, `@e2`, `@e3`... assigned in tree order

### What Gets a Ref

Only **interactive elements** get refs. These are elements with roles that indicate user interaction is possible:

| Role | Description | Example |
|---|---|---|
| `AXButton` | Clickable button | Toolbar buttons, dialog buttons |
| `AXTextField` | Single-line text input | Search fields, form inputs |
| `AXTextArea` | Multi-line text input | Document editors, code areas |
| `AXCheckBox` | Toggle checkbox | Settings toggles |
| `AXRadioButton` | Radio selection | Option groups |
| `AXPopUpButton` | Dropdown menu | Popup selectors |
| `AXComboBox` | Editable dropdown | Combo inputs |
| `AXSlider` | Value slider | Volume, brightness |
| `AXLink` | Clickable link | Hyperlinks in content |
| `AXMenuItem` | Menu item | Menu bar items, context menus |
| `AXMenuButton` | Menu trigger button | Dropdown triggers |
| `AXTab` | Tab selector | Tab bars |
| `AXTabGroup` | Tab container | Tabbed interfaces |
| `AXScrollArea` | Scrollable region | Content areas |
| `AXTable` | Data table | List/table views |
| `AXOutline` | Tree/outline view | Sidebar trees |
| `AXSwitch` | Toggle switch | On/off switches |
| `AXSearchField` | Search input | Search bars |
| `AXIncrementor` | Stepper/incrementor | Number steppers |

Non-interactive elements (labels, images, containers, separators) are shown as structural context but don't get refs.

## Using Refs

Once you have refs from a snapshot, interact directly:

```bash
# Click a button
agent-desktop click @e1

# Fill a text field (clear + type)
agent-desktop fill @e4 "Hello World"

# Type without clearing
agent-desktop type @e4 "additional text"

# Get element text
agent-desktop get text @e4
```

## Ref Lifecycle

**Refs are ephemeral.** They're valid from the moment a snapshot is taken until the UI changes significantly. Always re-snapshot after:

### Actions That Invalidate Refs

| Action | Why | Fix |
|---|---|---|
| Clicking a button | May change view, open dialog | Re-snapshot |
| Opening/closing a menu | Menu items appear/disappear | Re-snapshot |
| Switching tabs | Different content visible | Re-snapshot |
| Opening a dialog/sheet | Modal overlay changes tree | Re-snapshot |
| Closing a window | Window gone | Re-snapshot |
| App switching | Different app is frontmost | Re-snapshot |
| Scrolling (sometimes) | New elements enter viewport | Re-snapshot if needed |

### Safe Pattern

```bash
agent-desktop snapshot -i           # Take snapshot → get refs
agent-desktop click @e5             # Act on ref
agent-desktop snapshot -i           # ALWAYS re-snapshot after action
agent-desktop click @e10            # Use new refs
```

### Unsafe Pattern (Common Mistake)

```bash
agent-desktop snapshot -i           # Take snapshot
agent-desktop click @e5             # Opens a new view
agent-desktop click @e3             # ❌ WRONG — @e3 from old snapshot, may be stale
```

## Ref Resolution

When you use a ref (e.g., `click @e3`), the daemon resolves it to a screen position through this chain:

1. **AX Path re-traversal** (primary): Walk the stored role + index path from the app root to re-find the element. Gets current frame position.
2. **Stored coordinates** (fallback): Use the frame coordinates captured at snapshot time. Less reliable if the UI has shifted.
3. **Label search** (last resort): Search the accessibility tree by stored label text.

If resolution fails entirely, you get an actionable error:
```
Error [REF_NOT_FOUND]: Element @e3 not found. The UI may have changed.
Suggestion: Run `snapshot` to refresh element references.
```

## Best Practices

### 1. Always Start with a Snapshot

Never guess refs. Always snapshot first to discover what's available.

```bash
agent-desktop open "Finder"
agent-desktop wait 1000
agent-desktop snapshot -i   # Discover available elements
```

### 2. Re-Snapshot After Every Significant Action

Any action that changes the UI (clicking, opening menus, switching tabs) invalidates refs.

### 3. Use `-d` for Complex Apps

Apps like Xcode, VS Code, or Slack have deep accessibility trees. Use `-d 5` for faster results when you only need top-level elements.

```bash
agent-desktop snapshot -i -d 5   # Faster, less detail
agent-desktop snapshot -i -d 20  # Slower, more detail
```

### 4. Use `--app` for Multi-App Workflows

Target specific apps without switching focus:

```bash
agent-desktop snapshot -i --app "Finder"
agent-desktop snapshot -i --app "TextEdit"
```

### 5. Screenshots as Fallback

When the accessibility tree doesn't show what you need:

```bash
agent-desktop screenshot
# Visually inspect the screenshot
agent-desktop click 500 300   # Click by coordinates as fallback
```

### 6. Compact Mode for Token Savings

Use `-c` when you need to minimize token usage:

```bash
agent-desktop snapshot -i -c   # ~30% fewer tokens
```

## Troubleshooting

### "Element @eN not found"

The UI has changed since your last snapshot. Re-snapshot:
```bash
agent-desktop snapshot -i
```

### Snapshot returns very few elements

- The app may have poor accessibility support
- Try increasing depth: `agent-desktop snapshot -i -d 20`
- Try without `-i` to see the full tree: `agent-desktop snapshot -d 5`
- For Electron apps, use `--with-cdp` for richer access
- Fall back to screenshots + coordinate clicking

### Snapshot is slow (>3 seconds)

- Reduce depth: `agent-desktop snapshot -i -d 5`
- Target a specific app: `agent-desktop snapshot -i --app "Finder"`
- Complex apps (VS Code, Xcode) with many elements naturally take longer

### Click hits the wrong element

- Re-snapshot to get fresh coordinates
- Use `get text @eN` to verify the element is what you expect
- If the element has moved, its stored coordinates may be stale

### "Permission denied" errors

Run `agent-desktop status` and check for ❌ marks. See [permissions.md](permissions.md) for setup guide.
