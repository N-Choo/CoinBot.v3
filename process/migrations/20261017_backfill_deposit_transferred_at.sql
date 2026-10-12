-- Pre-existing confirmed deposits were already swept Main -> Trade (by the old
-- transfer-before-confirm flow, using a random clientOid). Mark them swept so
-- the new retry pass does not move their funds a second time.
UPDATE deposits
SET transferred_at = COALESCE(processed_at, NOW())
WHERE status = 'confirmed' AND transferred_at IS NULL;
