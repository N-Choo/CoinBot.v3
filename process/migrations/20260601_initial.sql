CREATE TABLE IF NOT EXISTS users (
    wallet_address TEXT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS uid UUID NOT NULL DEFAULT gen_random_uuid();
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_uid ON users (uid);

CREATE TABLE IF NOT EXISTS deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid),
    tx_hash TEXT NOT NULL,
    ticker VARCHAR(10) NOT NULL,
    amount TEXT NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    block_number BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_deposits_user_uid ON deposits (user_uid);
CREATE INDEX IF NOT EXISTS idx_deposits_status ON deposits (status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_deposits_tx_hash ON deposits (tx_hash);
