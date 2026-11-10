DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'withdrawals_status_check'
    ) THEN
        ALTER TABLE withdrawals
            ADD CONSTRAINT withdrawals_status_check
            CHECK (status IN ('pending', 'processing', 'submitted',
                              'completed', 'failed', 'expired'));
    END IF;
END $$;
