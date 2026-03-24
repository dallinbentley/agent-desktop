#!/bin/bash
# Template: Finder File Operations
# Usage: ./file-management.sh [directory-path]
# Example: ./file-management.sh ~/Documents

set -euo pipefail

TARGET_DIR="${1:-$HOME}"

echo "=== Opening Finder ==="
agent-computer open "Finder"
agent-computer wait 1000

echo "=== Navigating to $TARGET_DIR ==="
# Use Cmd+Shift+G to open "Go to Folder" dialog
agent-computer press cmd+shift+g
agent-computer wait 500
agent-computer snapshot -i

echo "=== Typing path ==="
# Find the text field in the Go to Folder dialog
echo "Find text_field ref from snapshot above, then run:"
echo "  agent-computer fill @eN \"$TARGET_DIR\""
echo "  agent-computer press enter"
echo "  agent-computer wait 1000"
echo "  agent-computer snapshot -i"

echo ""
echo "=== Taking snapshot of directory ==="
agent-computer wait 1500
agent-computer snapshot -i

echo "=== Taking screenshot ==="
agent-computer screenshot

echo "=== Done ==="
echo "Parse snapshot output above to interact with files."
echo "Double-click to open: agent-computer click @eN --double"
echo "Right-click for menu: agent-computer click @eN --right"
