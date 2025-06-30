-- Migration to refactor the theme system
-- 1. Modify the 'themes' table
-- 2. Modify the 'users' table

-- Step 1: Modify 'themes' table
-- First, rename the 'config' column to 'css'
ALTER TABLE themes RENAME COLUMN config TO css;

-- Then, change the data type of the new 'css' column to TEXT
-- Note: The exact syntax for changing column type can vary between SQL dialects.
-- This is a common approach for PostgreSQL.
ALTER TABLE themes ALTER COLUMN css TYPE TEXT;

-- Finally, drop the 'type' column
ALTER TABLE themes DROP COLUMN "type";


-- Step 2: Modify 'users' table
-- Add the new 'active_theme_id' column
ALTER TABLE users ADD COLUMN active_theme_id UUID;

-- Optional: Migrate existing data from old columns to the new one.
-- This example prioritizes the dark theme ID if both exist.
UPDATE users
SET active_theme_id = COALESCE(active_dark_theme_id, active_light_theme_id)
WHERE active_dark_theme_id IS NOT NULL OR active_light_theme_id IS NOT NULL;

-- Drop the old theme ID columns
ALTER TABLE users DROP COLUMN active_light_theme_id;
ALTER TABLE users DROP COLUMN active_dark_theme_id;