-- At most one live (pending or open) exit per position, so two replicas or a
-- retry cannot place a duplicate exit for the same entry.
CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_orders_active_close
    ON trade_orders (parent_order_id)
    WHERE parent_order_id IS NOT NULL AND status IN ('pending', 'open');

-- Track completion of a deposit's Main -> Trade sweep so a failed sweep can be
-- retried without re-crediting the user.
ALTER TABLE deposits ADD COLUMN IF NOT EXISTS transferred_at TIMESTAMPTZ;
