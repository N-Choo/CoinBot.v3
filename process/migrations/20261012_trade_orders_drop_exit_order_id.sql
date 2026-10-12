-- Replace the exchange-side OCO exit with an in-app order_sweeper: the exit
-- order id is no longer tracked on the entry row.
ALTER TABLE trade_orders DROP COLUMN IF EXISTS exit_order_id;
