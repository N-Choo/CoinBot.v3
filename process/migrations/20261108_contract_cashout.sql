ALTER TABLE contracts
    ADD COLUMN borrow_interest NUMERIC NOT NULL DEFAULT 0,
    ADD COLUMN platform_fee    NUMERIC NOT NULL DEFAULT 0,
    ADD COLUMN net_payout      NUMERIC,
    ADD COLUMN settlement_target VARCHAR(20)
        CHECK (settlement_target IS NULL
               OR settlement_target IN ('cancelled', 'completed')),
    ADD COLUMN settled_at      TIMESTAMPTZ;

ALTER TABLE trade_orders ADD COLUMN loan_apply_id TEXT;

CREATE TABLE margin_loans (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id        UUID NOT NULL REFERENCES contracts(id),
    entry_order_id     UUID NOT NULL REFERENCES trade_orders(id),
    currency           TEXT NOT NULL,
    principal          NUMERIC NOT NULL,
    outstanding        NUMERIC NOT NULL,
    loan_apply_id      TEXT,
    opened_at          TIMESTAMPTZ NOT NULL,
    position_closed_at TIMESTAMPTZ,
    interest_base      NUMERIC NOT NULL DEFAULT 0,
    interest_usdt      NUMERIC NOT NULL DEFAULT 0,
    repay_claimed_at   TIMESTAMPTZ,
    repay_buy_order_id TEXT,
    repay_order_no     TEXT,
    repaid_at          TIMESTAMPTZ,
    repaid_cost_usdt   NUMERIC NOT NULL DEFAULT 0,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (entry_order_id)
);

CREATE INDEX idx_margin_loans_open
    ON margin_loans (currency, opened_at)
    WHERE repaid_at IS NULL;
CREATE INDEX idx_margin_loans_repay
    ON margin_loans (position_closed_at)
    WHERE repaid_at IS NULL;
CREATE INDEX idx_margin_loans_contract ON margin_loans (contract_id);

CREATE TABLE margin_interest_charges (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency     TEXT NOT NULL,
    amount       NUMERIC NOT NULL,
    day_ratio    NUMERIC,
    created_time BIGINT NOT NULL,
    ingested_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (currency, created_time, amount)
);

CREATE TABLE interest_cursors (
    currency          TEXT PRIMARY KEY,
    last_created_time BIGINT NOT NULL,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE platform_fees (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id),
    user_uid    UUID NOT NULL REFERENCES users(uid),
    amount      NUMERIC NOT NULL,
    kind        TEXT NOT NULL DEFAULT 'profit',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_platform_fees_contract_kind
    ON platform_fees (contract_id, kind);
