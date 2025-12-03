CREATE TABLE IF NOT EXISTS accounts (
    account_id SERIAL PRIMARY KEY,
    owner_address BYTEA NOT NULL,
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
    status VARCHAR(32) NOT NULL CHECK (status IN ('ACCEPTED_ON_L2', 'ACCEPTED_ON_L1', 'FINALIZED')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    proven_at  TIMESTAMP,
    submitted_at TIMESTAMP
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
    status VARCHAR(32) NOT NULL CHECK (status IN ('ACCEPTED_ON_L2', 'ACCEPTED_ON_L1', 'FINALIZED')),
    batch_id INT REFERENCES batches(batch_id)
);

-- Speed up account balance lookups
CREATE INDEX IF NOT EXISTS idx_accounts_owner ON accounts(owner_address);


-- Speed up batch transaction lookup
CREATE INDEX IF NOT EXISTS idx_transactions_batch ON transactions(batch_id);

-- Speed up tx history lookups
CREATE INDEX IF NOT EXISTS idx_transactions_sender ON transactions(sender_address);
CREATE INDEX IF NOT EXISTS idx_transactions_recipient ON transactions(recipient_address);

-- Speed up lookups
CREATE INDEX IF NOT EXISTS idx_batches_active_status ON batches(status) WHERE status IN ('ACCEPTED_ON_L2', 'ACCEPTED_ON_L1');
