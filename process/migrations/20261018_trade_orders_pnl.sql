-- Realized PnL (quote currency, net of fees) for a closed round trip. Set on
-- close orders only; entries leave it NULL.
ALTER TABLE trade_orders ADD COLUMN IF NOT EXISTS pnl NUMERIC;

-- Currency an order's fee was charged in (e.g. USDT), so close-time PnL can
-- convert entry and exit fees into the contract's quote currency.
ALTER TABLE trade_orders ADD COLUMN IF NOT EXISTS fee_currency TEXT;
