#!/bin/bash
# Template: Navigate and Change a System Setting
# Usage: ./system-settings.sh "Setting Pane Name"
# Example: ./system-settings.sh "Appearance"

set -euo pipefail

PANE="${1:?Usage: $0 <setting-pane-name>}"

echo "=== Opening System Settings ==="
agent-desktop open "System Settings"
agent-desktop wait 2500

echo "=== Taking initial snapshot ==="
agent-desktop snapshot -i

echo "=== Searching for '$PANE' ==="
# Use the search field to navigate to the desired pane
# Parse snapshot output to find the search field ref, then:
echo "Find search_field ref from snapshot above, then run:"
echo "  agent-desktop fill @eN \"$PANE\""
echo "  agent-desktop wait 1000"
echo "  agent-desktop snapshot -i"
echo "  # Click the matching result"
echo "  agent-desktop click @eM"
echo "  agent-desktop wait 1000"
echo "  agent-desktop snapshot -i"
echo "  # Now interact with the setting controls"

echo ""
echo "=== Taking screenshot for reference ==="
agent-desktop screenshot

echo "=== Done ==="
