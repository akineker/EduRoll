#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# remove.sh - shut downs the simulation and leaves system resources
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo ""
echo "Shutting down containers..."
docker-compose down
echo "Removing all docker containers..."
docker system prune
echo "Removing all Forge compiles..."
forge clean
echo "Removing all Rust compiles..."
cargo clean
echo "Stopping colima..."
colima stop
echo "Completed."
echo ""

echo "  To restart the simulation:   bash scripts/run-fresh.sh"
echo ""