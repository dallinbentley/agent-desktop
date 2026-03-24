#!/bin/bash
# Template: Finder File Operations
# Usage: ./file-management.sh [directory-path]
# Example: ./file-management.sh ~/Documents

set -euo pipefail

TARGET_DIR="${1:-$HOME}"

echo "=== Opening Finder ==="
agent-desktop open "Finder"
agent-desktop wait 1000

echo "=== Navigating to $TARGET_DIR ==="
# Use Cmd+Shift+G to open "Go to Folder" dialog
agent-desktop press cmd+shift+g
agent-desktop wait 500
agent-desktop snapshot -i

echo "=== Typing path ==="
# Find the text field in the Go to Folder dialog
echo "Find text_field ref from snapshot above, then run:"
echo "  agent-desktop fill @eN \"$TARGET_DIR\""
echo "  agent-desktop press enter"
echo "  agent-desktop wait 1000"
echo "  agent-desktop snapshot -i"

echo ""
echo "=== Taking snapshot of directory ==="
agent-desktop wait 1500
agent-desktop snapshot -i

echo "=== Taking screenshot ==="
agent-desktop screenshot

echo "=== Done ==="
echo "Parse snapshot output above to interact with files."
echo "Double-click to open: agent-desktop click @eN --double"
echo "Right-click for menu: agent-desktop click @eN --right"
