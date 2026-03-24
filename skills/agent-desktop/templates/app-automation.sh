#!/bin/bash
# Template: Basic App Automation
# Usage: ./app-automation.sh "AppName" "action text"
# Example: ./app-automation.sh "TextEdit" "Hello World"

set -euo pipefail

APP_NAME="${1:?Usage: $0 <app-name> [text-to-type]}"
TEXT="${2:-}"

echo "=== Opening $APP_NAME ==="
agent-desktop open "$APP_NAME"
agent-desktop wait 1500

echo "=== Taking snapshot ==="
agent-desktop snapshot -i

if [ -n "$TEXT" ]; then
    echo "=== Finding text area and typing ==="
    # Note: In real usage, parse snapshot output to find the right ref
    # This is a template — adapt refs based on actual snapshot output
    echo "Snapshot taken. Parse output above to find text area ref."
    echo "Then run: agent-desktop fill @eN \"$TEXT\""
fi

echo "=== Taking screenshot ==="
agent-desktop screenshot

echo "=== Done ==="
