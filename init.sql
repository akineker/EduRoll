CREATE TABLE IF NOT EXISTS accounts (
    account_id SERIAL PRIMARY KEY,
    owner_address BYTEA NOT NULL,
    balance NUMERIC(20, 0) NOT NULL,
    merkle_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    tx_hash BYTEA PRIMARY KEY,
    sender_address BYTEA NOT NULL,
    recipient_address BYTEA NOT NULL,
    amount NUMERIC(20, 0) NOT NULL,
    status VARCHAR(10) NOT NULL
);

CREATE TABLE IF NOT EXISTS batches (
    batch_id SERIAL PRIMARY KEY,
    pre_state_root BYTEA NOT NULL,
    post_state_root BYTEA NOT NULL,
    zk_proof BYTEA NOT NULL,
    l1_tx_hash BYTEA,
    submission_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);