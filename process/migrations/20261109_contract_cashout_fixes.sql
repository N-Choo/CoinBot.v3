ALTER TABLE trade_orders ADD COLUMN IF NOT EXISTS borrow_size NUMERIC;

ALTER TABLE margin_loans ADD COLUMN IF NOT EXISTS repay_buy_client_oid TEXT;

ALTER TABLE margin_interest_charges
    DROP CONSTRAINT IF EXISTS margin_interest_charges_currency_created_time_amount_key;
ALTER TABLE margin_interest_charges
    ADD CONSTRAINT margin_interest_charges_currency_created_time_key
    UNIQUE (currency, created_time);
