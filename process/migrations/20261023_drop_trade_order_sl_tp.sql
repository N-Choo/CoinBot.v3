-- Destructive: old trade-engine binaries still INSERT tp_price/sl_price.
-- Stop/drain all old trade-engine instances before applying this migration.
ALTER TABLE trade_orders DROP COLUMN IF EXISTS tp_price;
ALTER TABLE trade_orders DROP COLUMN IF EXISTS sl_price;
