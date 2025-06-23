-- MIGRATION: Modify user table for username-based authentication

-- Step 1: Drop the email column
ALTER TABLE "users" DROP COLUMN "email";