INSERT INTO users (wallet_address, balance)
VALUES
    ('0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c', '50000'),
    ('0xdead000000000000000000000000000000000000', '25000'),
    ('0xabcd1234abcd1234abcd1234abcd1234abcd1234', '10000')
ON CONFLICT (wallet_address) DO NOTHING;

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1b',
    '{"nonce":"a1b2c3d4-e5f6-7890-abcd-ef1234567890","settings":{"Ticker":"BTC/USDT","Amount":"5000","StopLoss":"2.0","TakeProfit":"5.0"}}',
    'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    'BTC/USDT', '5000', 2.0, 5.0, 'active',
    NOW() - INTERVAL '3 days', NOW() - INTERVAL '3 days'
FROM users u WHERE u.wallet_address = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x2234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef2b',
    '{"nonce":"b2c3d4e5-f6a7-8901-bcde-f12345678901","settings":{"Ticker":"ETH/USDT","Amount":"2000","StopLoss":"1.5","TakeProfit":"4.0"}}',
    'b2c3d4e5-f6a7-8901-bcde-f12345678901',
    'ETH/USDT', '2000', 1.5, 4.0, 'active',
    NOW() - INTERVAL '2 days', NOW() - INTERVAL '2 days'
FROM users u WHERE u.wallet_address = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x3234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef3b',
    '{"nonce":"c3d4e5f6-a7b8-9012-cdef-123456789012","settings":{"Ticker":"SOL/USDT","Amount":"1000","StopLoss":"3.0","TakeProfit":"8.0"}}',
    'c3d4e5f6-a7b8-9012-cdef-123456789012',
    'SOL/USDT', '1000', 3.0, 8.0, 'active',
    NOW() - INTERVAL '1 day', NOW() - INTERVAL '1 day'
FROM users u WHERE u.wallet_address = '0xdead000000000000000000000000000000000000';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x4234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef4b',
    '{"nonce":"d4e5f6a7-b8c9-0123-defa-234567890123","settings":{"Ticker":"ADA/USDT","Amount":"750","StopLoss":"2.5","TakeProfit":"6.0"}}',
    'd4e5f6a7-b8c9-0123-defa-234567890123',
    'ADA/USDT', '750', 2.5, 6.0, 'completed',
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '5 days'
FROM users u WHERE u.wallet_address = '0xdead000000000000000000000000000000000000';

INSERT INTO contracts (user_uid, signature, message, nonce, ticker, amount, sl_pct, tp_pct, status, created_at, updated_at)
SELECT
    u.uid,
    '0x5234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5b',
    '{"nonce":"e5f6a7b8-c9d0-1234-efab-345678901234","settings":{"Ticker":"DOT/USDT","Amount":"300","StopLoss":"4.0","TakeProfit":"10.0"}}',
    'e5f6a7b8-c9d0-1234-efab-345678901234',
    'DOT/USDT', '300', 4.0, 10.0, 'cancelled',
    NOW() - INTERVAL '14 days', NOW() - INTERVAL '10 days'
FROM users u WHERE u.wallet_address = '0xabcd1234abcd1234abcd1234abcd1234abcd1234';
