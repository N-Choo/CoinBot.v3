-- `snap_balance` was an unused balance snapshot recorded at signing time.
ALTER TABLE contracts DROP COLUMN IF EXISTS snap_balance;
