CREATE INDEX IF NOT EXISTS idx_withdrawals_expiring
    ON withdrawals (expires_at)
    WHERE status IN ('pending', 'processing');
