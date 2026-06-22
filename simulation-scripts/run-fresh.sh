#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# run-fresh.sh - full fresh-deploy cycle.
#
#   1. Wipe both Postgres and Anvil volumes (clean slate).
#   2. Bring up infra (postgres, anvil).
#   3. Run seed_accounts to populate 1000 deterministic accounts AND capture
#      the resulting Poseidon genesis state root.
#   4. Deploy L1 contracts at that genesis root.
#   5. Start the L2 services + test_client.
#
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# L1 signer for the local devnet. Defaults to the PUBLIC Anvil/Foundry dev
: "${L1_PRIVATE_KEY:=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}"

echo "[1/6] Starting colima..."
colima status || colima start --cpu 4 --memory 8 --runtime docker

echo "[2/6] Rebuilding all service images (cached layers reused)..."

SERVICES="seed_accounts sequencer prover submitter archiver test_client"
for svc in $SERVICES; do
    echo "  -> building $svc"
    if ! docker-compose build "$svc"; then
        echo "ERROR: docker-compose build failed for service '$svc'" >&2
        exit 1
    fi
done

echo "[3/6] Starting Postgres and Anvil..."
docker-compose up -d postgres anvil
# Wait for Postgres
for i in {1..30}; do
    if docker-compose exec -T postgres pg_isready -U eduroll_user >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

echo "[4/6] Seeding 1000 accounts (this prints the genesis state root)..."
SEED_OUT="$(docker-compose run --rm seed_accounts)"
echo "${SEED_OUT}"
GENESIS_ROOT="$(printf '%s\n' "${SEED_OUT}" | grep -oE 'GENESIS_ROOT=0x[0-9a-fA-F]{64}' | tail -1 | cut -d= -f2)"
if [[ -z "${GENESIS_ROOT}" ]]; then
    echo "ERROR: could not capture GENESIS_ROOT from seed_accounts output" >&2
    exit 1
fi
echo ">>> Genesis state root: ${GENESIS_ROOT}"

echo "[5/6] Deploying L1 contracts at the genesis root..."
INITIAL_STATE_ROOT="${GENESIS_ROOT}" \
L1_PRIVATE_KEY="${L1_PRIVATE_KEY}" \
    forge script script/Deploy.s.sol:Deploy \
    --rpc-url http://localhost:8545 \
    --broadcast

# Sync deployed addresses into .env + verify the deploy actually landed
DEPLOY_RPC="http://localhost:8545"

CHAIN_ID="$(cast chain-id --rpc-url "${DEPLOY_RPC}" 2>/dev/null || true)"
if [[ -z "${CHAIN_ID}" ]]; then
    echo "ERROR: cannot reach the deploy RPC ${DEPLOY_RPC}." >&2
    echo "       Is Anvil up and does Colima forward its port to the host?" >&2
    echo "       Check with: cast block-number --rpc-url ${DEPLOY_RPC}" >&2
    exit 1
fi
if [[ "${CHAIN_ID}" != "1337" ]]; then
    echo "WARNING: deploy RPC chain id is ${CHAIN_ID}, but the services' Anvil is configured for 1337 -" >&2
    echo "         the deploy may be hitting a different node than the submitter (anvil:8545)." >&2
fi

BROADCAST_FILE="broadcast/Deploy.s.sol/${CHAIN_ID}/run-latest.json"
if [[ ! -f "${BROADCAST_FILE}" ]]; then
    echo "ERROR: deploy broadcast not found at ${BROADCAST_FILE} - the deploy did not broadcast." >&2
    exit 1
fi

# Pull addresses from the forge broadcast
read -r TOKEN_ADDR BRIDGE_ADDR ROLLUP_ADDR < <(python3 - "${BROADCAST_FILE}" <<'PY'
import json, sys
addrs = {}
for t in json.load(open(sys.argv[1])).get("transactions", []):
    name, addr = t.get("contractName"), t.get("contractAddress")
    if name and addr:
        addrs[name] = addr
print(addrs.get("MockToken", ""), addrs.get("BridgeERC20", ""), addrs.get("Rollup", ""))
PY
)
if [[ -z "${ROLLUP_ADDR}" ]]; then
    echo "ERROR: could not parse the Rollup address from ${BROADCAST_FILE}." >&2
    exit 1
fi
echo ">>> Deployed: TOKEN=${TOKEN_ADDR}  BRIDGE=${BRIDGE_ADDR}  ROLLUP=${ROLLUP_ADDR}"

set_env_var() {
    local key="$1" val="$2"
    if grep -qE "^${key}=" .env 2>/dev/null; then
        awk -v k="${key}" -v v="${val}" -F= '$1==k{print k"="v; next} {print}' .env > .env.tmp && mv .env.tmp .env
    else
        printf '%s=%s\n' "${key}" "${val}" >> .env
    fi
}
set_env_var ROLLUP_ADDRESS "${ROLLUP_ADDR}"
set_env_var BRIDGE_ADDRESS "${BRIDGE_ADDR}"
set_env_var TOKEN_ADDRESS  "${TOKEN_ADDR}"
echo ">>> .env updated with the freshly deployed addresses."

# Guard: refuse to start services if the Rollup has no bytecode on the deploy
ROLLUP_CODE="$(cast code "${ROLLUP_ADDR}" --rpc-url "${DEPLOY_RPC}" 2>/dev/null || true)"
if [[ -z "${ROLLUP_CODE}" || "${ROLLUP_CODE}" == "0x" ]]; then
    echo "ERROR: no contract code at Rollup ${ROLLUP_ADDR} on ${DEPLOY_RPC}." >&2
    echo "       The deploy did not land on this Anvil - aborting before starting services." >&2
    exit 1
fi
echo ">>> Verified: Rollup bytecode present on chain ${CHAIN_ID}."

echo "[6/6] Starting L2 services + test_client..."
docker-compose up -d sequencer prover submitter archiver
docker-compose up -d test_client

echo ""
echo "DONE."
echo "  Tail logs:    docker-compose logs -f"
echo "  Prover only:  docker-compose logs -f prover"
echo "  Stop (keep state):  docker-compose down"
