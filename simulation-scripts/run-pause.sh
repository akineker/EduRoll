#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# run-pause.sh — Pauses the system
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo ""
echo "Shutting down containers..."
docker-compose down
echo "Stopping Colima services..."
colima stop
echo "To resume the simulation:   bash simulation-scripts/run-resume.sh"
echo ""