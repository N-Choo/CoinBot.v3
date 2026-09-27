CREATE TABLE IF NOT EXISTS trade_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id),
    user_uid UUID NOT NULL REFERENCES users(uid),
    ticker VARCHAR(20) NOT NULL,
    side VARCHAR(10) NOT NULL,
    comment TEXT NOT NULL,
    exchange_order_id TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    requested_qty TEXT NOT NULL,
    filled_qty TEXT NOT NULL DEFAULT '0',
    fill_price TEXT,
    fee TEXT NOT NULL DEFAULT '0',
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_orders_comment ON trade_orders (comment);
CREATE INDEX IF NOT EXISTS idx_trade_orders_contract ON trade_orders (contract_id);
CREATE INDEX IF NOT EXISTS idx_trade_orders_user ON trade_orders (user_uid);
CREATE INDEX IF NOT EXISTS idx_trade_orders_status ON trade_orders (status);
