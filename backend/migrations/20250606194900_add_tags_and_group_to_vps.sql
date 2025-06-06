-- Add tags and group columns to the vps table
ALTER TABLE vps
ADD COLUMN tags VARCHAR(255),
ADD COLUMN "group" VARCHAR(255);