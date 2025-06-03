-- Add migration script here
-- Remove load average columns from the performance_metrics table

ALTER TABLE performance_metrics
DROP COLUMN IF EXISTS load_average_one_min,
DROP COLUMN IF EXISTS load_average_five_min,
DROP COLUMN IF EXISTS load_average_fifteen_min;
