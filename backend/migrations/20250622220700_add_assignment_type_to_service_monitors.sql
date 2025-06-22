-- Add assignment_type to service_monitors table
ALTER TABLE service_monitors
ADD COLUMN assignment_type VARCHAR(255) NOT NULL DEFAULT 'INCLUSIVE';