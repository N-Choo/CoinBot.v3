CREATE TABLE IF NOT EXISTS withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid),
    destination TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USDT',
    chain TEXT NOT NULL DEFAULT 'eth',
    amount NUMERIC NOT NULL,
    fee NUMERIC NOT NULL,
    total NUMERIC NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    nonce TEXT NOT NULL UNIQUE,
    message TEXT NOT NULL,
    signature TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    claimed_at TIMESTAMPTZ,
    attempted_at TIMESTAMPTZ,
    attempted_amount NUMERIC,
    attempted_fee NUMERIC,
    kucoin_withdrawal_id TEXT,
    tx_hash TEXT,
    actual_fee NUMERIC,
    failure_reason TEXT,
    submitted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_withdrawals_status_created
    ON withdrawals (status, created_at);
CREATE INDEX IF NOT EXISTS idx_withdrawals_user_created
    ON withdrawals (user_uid, created_at DESC);

CREATE TABLE IF NOT EXISTS ledger_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid),
    kind TEXT NOT NULL,
    delta_available NUMERIC NOT NULL,
    delta_locked NUMERIC NOT NULL,
    balance_after NUMERIC NOT NULL,
    locked_after NUMERIC NOT NULL,
    ref_type TEXT NOT NULL,
    ref_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ledger_user_created
    ON ledger_entries (user_uid, created_at DESC);
