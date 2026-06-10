CREATE TABLE IF NOT EXISTS contracts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uid UUID NOT NULL REFERENCES users(uid),
    signature TEXT NOT NULL,
    message TEXT NOT NULL,
    nonce TEXT NOT NULL,

    ticker VARCHAR(10) NOT NULL,
    amount TEXT NOT NULL,
    sl_pct REAL NOT NULL,
    tp_pct REAL NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_contracts_nonce ON contracts (nonce);
CREATE INDEX IF NOT EXISTS idx_contracts_user_uid ON contracts (user_uid);
CREATE INDEX IF NOT EXISTS idx_contracts_ticker ON contracts (ticker);
CREATE INDEX IF NOT EXISTS idx_contracts_status ON contracts (status);
CREATE INDEX IF NOT EXISTS idx_contracts_created_at ON contracts (created_at DESC);
