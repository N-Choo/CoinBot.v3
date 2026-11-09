ALTER TABLE margin_loans
    ADD COLUMN IF NOT EXISTS repay_attempted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS repay_attempted_amount NUMERIC,
    ADD COLUMN IF NOT EXISTS repay_attempted_cost_usdt NUMERIC;
