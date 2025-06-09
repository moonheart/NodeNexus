-- Migration: Create vps_renewal_info table with integrated reminder fields

-- Table: vps_renewal_info
-- Stores detailed renewal information for each VPS, including reminder status.
CREATE TABLE IF NOT EXISTS vps_renewal_info (
    vps_id INTEGER NOT NULL PRIMARY KEY,
    renewal_cycle TEXT, -- e.g., "monthly", "annually", "custom_days"
    renewal_cycle_custom_days INTEGER, -- if renewal_cycle is "custom_days"
    renewal_price DOUBLE PRECISION,
    renewal_currency TEXT, -- e.g., "USD", "CNY"
    next_renewal_date TIMESTAMPTZ,
    last_renewal_date TIMESTAMPTZ,
    service_start_date TIMESTAMPTZ,
    payment_method TEXT,
    auto_renew_enabled BOOLEAN DEFAULT false,
    renewal_notes TEXT,
    reminder_active BOOLEAN DEFAULT false, -- True if a reminder is currently active for the next_renewal_date
    last_reminder_generated_at TIMESTAMPTZ, -- Timestamp of when the last reminder was generated for the current next_renewal_date
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (vps_id) REFERENCES vps(id) ON DELETE CASCADE
);

-- Create a trigger to automatically update 'updated_at' timestamp
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_vps_renewal_info_updated_at
BEFORE UPDATE ON vps_renewal_info
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();

-- Add default setting for renewal reminder lead days
-- This setting determines how many days in advance a reminder should become active.
INSERT INTO settings (key, value, updated_at)
VALUES ('renewal_reminder_lead_days', '{"value": 7}', CURRENT_TIMESTAMP)
ON CONFLICT (key) DO UPDATE
SET value = '{"value": 7}', updated_at = CURRENT_TIMESTAMP;