# Permissions Setup

agent-desktop requires two macOS system permissions to function. This guide covers setup, troubleshooting, and terminal-specific instructions.

**Related**: [SKILL.md](../SKILL.md) for quick start, [commands.md](commands.md) for `status` command details.

## Required Permissions

| Permission | Required For | API |
|---|---|---|
| **Accessibility** | Reading UI elements, simulating clicks/keyboard | AXUIElement, CGEvent |
| **Screen Recording** | Taking screenshots | ScreenCaptureKit |

## Quick Check

```bash
agent-desktop status
# Accessibility: ✅ granted (or ❌ denied)
# Screen Recording: ✅ granted (or ❌ denied)
```

## Granting Accessibility Permission

1. Open **System Settings** → **Privacy & Security** → **Accessibility**
2. Click the **+** button (or lock icon to unlock first)
3. Add your **terminal application**:
   - **Terminal.app**: `/Applications/Utilities/Terminal.app`
   - **iTerm2**: `/Applications/iTerm.app`
   - **Warp**: `/Applications/Warp.app`
   - **Alacritty**: `/Applications/Alacritty.app`
   - **Kitty**: `/Applications/kitty.app`
   - **VS Code Terminal**: Add `/Applications/Visual Studio Code.app`
4. Ensure the toggle is **ON**
5. You may need to **restart your terminal** for changes to take effect

### Programmatic check

The daemon uses `AXIsProcessTrusted()` to check accessibility permission. This returns `true` if the process (or its parent terminal) has been granted access.

## Granting Screen Recording Permission

1. Open **System Settings** → **Privacy & Security** → **Screen Recording**
2. Add your **terminal application** (same as above)
3. Toggle **ON**
4. **Restart your terminal** — Screen Recording permission changes require app restart

### Note on Screen Recording

Screen Recording is only needed for the `screenshot` command. All other commands (snapshot, click, type, etc.) work with just Accessibility permission.

## Troubleshooting

### "Accessibility permission required" but I already granted it

- **Restart your terminal.** Some permission changes require a fresh process.
- **Check you added the right app.** If using VS Code's integrated terminal, you need to add VS Code, not Terminal.app.
- **Check the toggle is ON.** The app must be listed AND enabled.
- **Try removing and re-adding.** Sometimes toggling off, removing, re-adding, and toggling on fixes stale grants.

### Permission works in Terminal.app but not iTerm2

Each terminal app needs its own grant. The permission is tied to the app that spawns the process, not the shell itself.

### Running via SSH or tmux

When running via SSH, tmux, or screen:
- The **sshd** process or **tmux server** may need the permission instead of (or in addition to) your terminal app
- For tmux: grant permission to the app that started the tmux server
- For SSH: grant to `/usr/sbin/sshd` or run `agent-desktop` from a local terminal first to establish permissions

### Reset permissions (nuclear option)

If permissions are in a weird state, reset and re-grant:

```bash
# Reset Accessibility permissions for agent-desktop-daemon
tccutil reset Accessibility com.agent-desktop.daemon

# Reset Screen Recording
tccutil reset ScreenCapture com.agent-desktop.daemon
```

Then re-add the terminal app in System Settings.

## Headless / CI Environments

For automated environments without a GUI session:
- Accessibility and Screen Recording permissions must be pre-provisioned via MDM profiles or `tccutil`
- Screenshot functionality requires an active login session (not just an SSH connection)
- Consider using `--json` mode for programmatic output parsing
