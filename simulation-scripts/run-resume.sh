#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# run-resume.sh — bring everything back up using the existing DB + Anvil state.
#
# Use after `docker-compose down` (NO `-v`). Postgres data and Anvil chain
# state survive in named volumes; this just starts every container, and the
# services resume where they left off:
#   - sequencer rebuilds in-memory state from `accounts`
#   - prover picks up any leftover PENDING_PROOF batches
#   - submitter picks up any leftover PROVEN batches
#   - test_client refreshes nonces from /accounts and resumes signing
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "Starting all services (resuming from existing volumes)..."
colima start
docker-compose up -d

echo ""
echo "DONE."
echo "  docker-compose ps"
echo "  docker-compose logs -f"
