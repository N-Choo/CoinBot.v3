-- locked_balance = the user's contracts' allocated funds (what the sign step locks).
INSERT INTO users (wallet_address, balance, locked_balance)
VALUES
    ('0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c', '50000', '7000'),
    ('0xdead000000000000000000000000000000000000', '25000', '1750'),
    ('0xabcd1234abcd1234abcd1234abcd1234abcd1234', '10000', '300'),
    ('0x2300dc555ba1dda773935a2fac059e3c8f1d14a7', '50000', '5000')
ON CONFLICT (wallet_address) DO NOTHING;

-- total_traded_fund = sum of the contract's order funds (the engine accumulates
-- each order's fund into it); available_fund is back at init_fund because filled
-- orders credit their proceeds back to the contract.

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1b',
    '{"nonce":"a1b2c3d4-e5f6-7890-abcd-ef1234567890","settings":{"Ticker":"BTC/USDT","Amount":"5000","StopLoss":"2.0","TakeProfit":"5.0"}}',
    'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    'BTC/USDT', u.balance, '5000', '5000', '7476', 2.0, 5.0, 'active',
    NOW() - INTERVAL '3 days', NOW() - INTERVAL '3 days'
FROM users u WHERE u.wallet_address = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x2234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef2b',
    '{"nonce":"b2c3d4e5-f6a7-8901-bcde-f12345678901","settings":{"Ticker":"ETH/USDT","Amount":"2000","StopLoss":"1.5","TakeProfit":"4.0"}}',
    'b2c3d4e5-f6a7-8901-bcde-f12345678901',
    'ETH/USDT', u.balance, '2000', '2000', '2000', 1.5, 4.0, 'active',
    NOW() - INTERVAL '2 days', NOW() - INTERVAL '2 days'
FROM users u WHERE u.wallet_address = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x3234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef3b',
    '{"nonce":"c3d4e5f6-a7b8-9012-cdef-123456789012","settings":{"Ticker":"SOL/USDT","Amount":"1000","StopLoss":"3.0","TakeProfit":"8.0"}}',
    'c3d4e5f6-a7b8-9012-cdef-123456789012',
    'SOL/USDT', u.balance, '1000', '1000', '990', 3.0, 8.0, 'active',
    NOW() - INTERVAL '1 day', NOW() - INTERVAL '1 day'
FROM users u WHERE u.wallet_address = '0xdead000000000000000000000000000000000000';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x4234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef4b',
    '{"nonce":"d4e5f6a7-b8c9-0123-defa-234567890123","settings":{"Ticker":"ADA/USDT","Amount":"750","StopLoss":"2.5","TakeProfit":"6.0"}}',
    'd4e5f6a7-b8c9-0123-defa-234567890123',
    'ADA/USDT', u.balance, '750', '750', '0', 2.5, 6.0, 'completed',
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '5 days'
FROM users u WHERE u.wallet_address = '0xdead000000000000000000000000000000000000';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x5234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5b',
    '{"nonce":"e5f6a7b8-c9d0-1234-efab-345678901234","settings":{"Ticker":"DOT/USDT","Amount":"300","StopLoss":"4.0","TakeProfit":"10.0"}}',
    'e5f6a7b8-c9d0-1234-efab-345678901234',
    'DOT/USDT', u.balance, '300', '300', '0', 4.0, 10.0, 'cancelled',
    NOW() - INTERVAL '14 days', NOW() - INTERVAL '10 days'
FROM users u WHERE u.wallet_address = '0xabcd1234abcd1234abcd1234abcd1234abcd1234';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, snap_balance, init_fund, available_fund, total_traded_fund, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x6234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef6b',
    '{"nonce":"f6a7b8c9-d0e1-2345-fabc-456789012345","settings":{"Ticker":"BTC/USDT","Amount":"5000","StopLoss":"2.0","TakeProfit":"5.0"}}',
    'f6a7b8c9-d0e1-2345-fabc-456789012345',
    'BTC/USDT', u.balance, '5000', '5000', '14118', 2.0, 5.0, 'active',
    NOW() - INTERVAL '1 day', NOW() - INTERVAL '1 day'
FROM users u WHERE u.wallet_address = '0x2300dc555ba1dda773935a2fac059e3c8f1d14a7';

-- Trade orders. `filled` rows carry a close price (fill_price); `open` rows are
-- still working (no fill_price, filled_qty = 0) and have no exchange_order_id, so
-- the sweeper leaves them alone.
INSERT INTO trade_orders (contract_id, user_uid, ticker, side, action, comment, status, requested_qty, filled_qty, fill_price, fee, funds, created_at, updated_at)
SELECT c.id, c.user_uid, c.ticker, v.side, v.action, v.comment, v.status,
       v.qty,
       CASE WHEN v.status = 'filled' THEN v.qty    ELSE '0'  END,
       CASE WHEN v.status = 'filled' THEN v.price  ELSE NULL END,
       CASE WHEN v.status = 'filled' THEN v.fee    ELSE '0'  END,
       v.funds, NOW() - v.age, NOW() - v.age
FROM contracts c
JOIN (VALUES
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'buy',  'open',  'seed-order-1', 'filled', '0.0600', '82500', '1.20', '4950', INTERVAL '3 days'),
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'sell', 'close', 'seed-order-2', 'filled', '0.0300', '84200', '0.80', '2526', INTERVAL '2 days'),
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'buy',  'open',  'seed-order-3', 'filled', '0.8000', '2500',  '0.90', '2000', INTERVAL '2 days'),
    ('c3d4e5f6-a7b8-9012-cdef-123456789012', 'buy',  'open',  'seed-order-4', 'filled', '9.0000', '110',   '0.40', '990',  INTERVAL '1 day'),
    ('f6a7b8c9-d0e1-2345-fabc-456789012345', 'buy',  'open',  'seed-order-5', 'filled', '0.0600', '82600', '1.20', '4956', INTERVAL '22 hours'),
    ('f6a7b8c9-d0e1-2345-fabc-456789012345', 'sell', 'close', 'seed-order-6', 'filled', '0.0400', '83500', '0.90', '3340', INTERVAL '18 hours'),
    ('f6a7b8c9-d0e1-2345-fabc-456789012345', 'buy',  'open',  'seed-order-7', 'open',   '0.0500', '82800', '0.00', '4140', INTERVAL '10 hours'),
    ('f6a7b8c9-d0e1-2345-fabc-456789012345', 'sell', 'close', 'seed-order-8', 'open',   '0.0200', '84100', '0.00', '1682', INTERVAL '3 hours')
) AS v(nonce, side, action, comment, status, qty, price, fee, funds, age)
  ON c.nonce = v.nonce;
