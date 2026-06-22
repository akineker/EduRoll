#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# build-circuit.sh — (re)build the ZK artifacts after a circuit change.
#
#   1. Compile circuits/circom/transfer_eddsa_20.circom  -> r1cs + wasm
#   2. Groth16 trusted setup (phase-2 init from the committed pot21 ptau)
#   3. Phase-2 contribution  (randomises delta — do NOT skip; the un-contributed
#      key has delta == gamma == generator, i.e. forgeable proofs)
#   4. Export verification key + the Solidity verifier -> src/Verifier.sol
#
# ---------------------------------------------------------------------------
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

CIRCUIT="circuits/circom/transfer_eddsa_20.circom"
BUILD="circuits/build"
PTAU="$BUILD/pot21_final.ptau"
NAME="transfer_eddsa_20"

command -v circom  >/dev/null || { echo "ERROR: circom not found";  exit 1; }
command -v snarkjs >/dev/null || { echo "ERROR: snarkjs not found"; exit 1; }
[ -f "$PTAU" ] || { echo "ERROR: missing $PTAU (powers of tau)";    exit 1; }

echo "[1/4] Compiling $CIRCUIT ..."
circom "$CIRCUIT" --r1cs --wasm --sym -l . -o "$BUILD"

echo "[2/4] Groth16 setup (phase-2 init) ..."
snarkjs groth16 setup "$BUILD/$NAME.r1cs" "$PTAU" "$BUILD/${NAME}_0000.zkey"

echo "[3/4] Phase-2 contribution (randomises delta) ..."
snarkjs zkey contribute "$BUILD/${NAME}_0000.zkey" "$BUILD/transfer_final.zkey" \
    --name="eduroll-$(date +%s)" -e="eduroll-entropy-$(date +%s)-${RANDOM}"

echo "[4/4] Exporting verification key + Solidity verifier ..."
snarkjs zkey export verificationkey "$BUILD/transfer_final.zkey" "$BUILD/verification_key.json"
snarkjs zkey export solidityverifier  "$BUILD/transfer_final.zkey" src/Verifier.sol

echo ""
echo "DONE."
echo "  zkey     : $BUILD/transfer_final.zkey"
echo "  wasm     : $BUILD/${NAME}_js/${NAME}.wasm"
echo "  verifier : src/Verifier.sol  (public inputs = circuit's public signals)"
echo "  Next     : bash ./scripts/run-fresh.sh"
