-- Store money/quantity values as NUMERIC instead of TEXT so the database
-- enforces numeric integrity (no malformed strings, exact decimal arithmetic).

-- users
ALTER TABLE users ALTER COLUMN balance DROP DEFAULT;
ALTER TABLE users ALTER COLUMN locked_balance DROP DEFAULT;
ALTER TABLE users
    ALTER COLUMN balance TYPE NUMERIC USING balance::numeric,
    ALTER COLUMN locked_balance TYPE NUMERIC USING locked_balance::numeric;
ALTER TABLE users
    ALTER COLUMN balance SET DEFAULT 0,
    ALTER COLUMN locked_balance SET DEFAULT 0;

-- contracts
ALTER TABLE contracts ALTER COLUMN snap_balance DROP DEFAULT;
ALTER TABLE contracts ALTER COLUMN available_fund DROP DEFAULT;
ALTER TABLE contracts ALTER COLUMN total_traded_fund DROP DEFAULT;
ALTER TABLE contracts
    ALTER COLUMN init_fund TYPE NUMERIC USING init_fund::numeric,
    ALTER COLUMN snap_balance TYPE NUMERIC USING snap_balance::numeric,
    ALTER COLUMN available_fund TYPE NUMERIC USING available_fund::numeric,
    ALTER COLUMN total_traded_fund TYPE NUMERIC USING total_traded_fund::numeric;
ALTER TABLE contracts
    ALTER COLUMN snap_balance SET DEFAULT 0,
    ALTER COLUMN available_fund SET DEFAULT 0,
    ALTER COLUMN total_traded_fund SET DEFAULT 0;

-- deposits
ALTER TABLE deposits
    ALTER COLUMN amount TYPE NUMERIC USING amount::numeric;

-- trade_orders
ALTER TABLE trade_orders ALTER COLUMN filled_qty DROP DEFAULT;
ALTER TABLE trade_orders ALTER COLUMN fee DROP DEFAULT;
ALTER TABLE trade_orders ALTER COLUMN funds DROP DEFAULT;
ALTER TABLE trade_orders
    ALTER COLUMN requested_qty TYPE NUMERIC USING requested_qty::numeric,
    ALTER COLUMN filled_qty    TYPE NUMERIC USING filled_qty::numeric,
    ALTER COLUMN fill_price    TYPE NUMERIC USING fill_price::numeric,
    ALTER COLUMN fee           TYPE NUMERIC USING fee::numeric,
    ALTER COLUMN funds         TYPE NUMERIC USING funds::numeric,
    ALTER COLUMN tp_price      TYPE NUMERIC USING tp_price::numeric,
    ALTER COLUMN sl_price      TYPE NUMERIC USING sl_price::numeric;
ALTER TABLE trade_orders
    ALTER COLUMN filled_qty SET DEFAULT 0,
    ALTER COLUMN fee SET DEFAULT 0,
    ALTER COLUMN funds SET DEFAULT 0;
