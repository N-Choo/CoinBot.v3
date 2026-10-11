-- Allow more than one order per contract: `comment` is now a per-attempt client
-- order id, not the contract id.
DROP INDEX IF EXISTS idx_trade_orders_comment;
