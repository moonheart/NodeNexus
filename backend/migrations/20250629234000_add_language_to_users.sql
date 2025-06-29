-- Add language preference column to users table
ALTER TABLE "users" ADD COLUMN "language" VARCHAR(20) NOT NULL DEFAULT 'auto';