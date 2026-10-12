-- Link a close (exit) order to the entry order it protects, so the order
-- sweeper can tell which filled entries are still open positions.
ALTER TABLE trade_orders ADD COLUMN IF NOT EXISTS parent_order_id UUID REFERENCES trade_orders(id);
CREATE INDEX IF NOT EXISTS idx_trade_orders_parent ON trade_orders (parent_order_id);
