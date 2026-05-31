CREATE TABLE IF NOT EXISTS accounts (
    account_id SERIAL PRIMARY KEY,
    owner_address BYTEA NOT NULL,
    l2_pubkey_x BYTEA NOT NULL, 
    l2_pubkey_y BYTEA NOT NULL,
    balance NUMERIC(78, 0) NOT NULL,
    nonce BIGINT NOT NULL DEFAULT 0,
    leaf_index BIGINT NOT NULL, 
    CONSTRAINT uq_accounts_owner UNIQUE (owner_address),
    CONSTRAINT uq_leaf_index UNIQUE (leaf_index)
);

CREATE TABLE IF NOT EXISTS batches (
    batch_id SERIAL PRIMARY KEY,
    pre_state_root_poseidon BYTEA NOT NULL,
    post_state_root_poseidon BYTEA NOT NULL,
    pre_state_root_keccak   BYTEA NOT NULL,
    post_state_root_keccak  BYTEA NOT NULL,
    zk_proof BYTEA,
    l1_tx_hash BYTEA,
    l1_block_number BIGINT,
    status VARCHAR(32) NOT NULL CHECK (status IN (
        'PENDING_PROOF',
        'PROVEN',
        'SUBMITTED_TO_L1',
        'FINALIZED'
    )),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    proven_at    TIMESTAMP,
    submitted_at TIMESTAMP,
    finalized_at TIMESTAMP,
    archived_at  TIMESTAMP,
    archive_uri  TEXT
);

CREATE TABLE IF NOT EXISTS transactions (
    tx_hash BYTEA PRIMARY KEY,
    signature BYTEA NOT NULL,
    sender_address BYTEA NOT NULL,
    recipient_address BYTEA NOT NULL,
    amount NUMERIC(78, 0) NOT NULL,
    nonce BIGINT NOT NULL, 
    fee NUMERIC(78, 0) NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        status VARCHAR(32) NOT NULL CHECK (status IN (
        'PENDING',
        'ACCEPTED_ON_L2',
        'FINALIZED'
    )),
    batch_id INT REFERENCES batches(batch_id)
);

-- Per-batch snapshot of the PRE-batch account state.
-- Populated by the sequencer in the same DB transaction as the batch insert,
-- BEFORE the accounts table is updated. Lets the prover (or any other
-- consumer) reconstruct the exact pre-batch tree later.
CREATE TABLE IF NOT EXISTS batch_snapshots (
    snapshot_id   SERIAL PRIMARY KEY,
    batch_id      INT NOT NULL REFERENCES batches(batch_id) ON DELETE CASCADE,
    leaf_index    BIGINT NOT NULL,
    owner_address BYTEA NOT NULL,
    l2_pubkey_x   BYTEA NOT NULL,
    l2_pubkey_y   BYTEA NOT NULL,
    pre_balance   NUMERIC(78, 0) NOT NULL,
    pre_nonce     BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_batch_snapshots_batch ON batch_snapshots(batch_id);

-- OPTIMIZATION INDEXES
-- Speed up account balance lookups
CREATE INDEX IF NOT EXISTS idx_accounts_owner ON accounts(owner_address);

-- Speed up batch transaction lookup
CREATE INDEX IF NOT EXISTS idx_transactions_batch ON transactions(batch_id);

-- Speed up tx history lookups
CREATE INDEX IF NOT EXISTS idx_transactions_sender ON transactions(sender_address);
CREATE INDEX IF NOT EXISTS idx_transactions_recipient ON transactions(recipient_address);

-- Speed up lookups
CREATE INDEX IF NOT EXISTS idx_batches_active_status ON batches(status) 
    WHERE status IN ('PENDING_PROOF', 'PROVEN', 'SUBMITTED_TO_L1');

CREATE INDEX IF NOT EXISTS idx_txs_pending ON transactions(status) 
    WHERE status = 'PENDING';


-- Accounts are now populated by the `seed_accounts` binary on fresh-DB startup
-- (1000 deterministic accounts, real BabyJubJub keypairs).
-- See tools/accounts/src/bin/seed_accounts.rs.