-- Modify vps table to allow NULL for ip_address
ALTER TABLE vps
ALTER COLUMN ip_address DROP NOT NULL;