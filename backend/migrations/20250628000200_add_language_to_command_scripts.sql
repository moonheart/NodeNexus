-- Add language column to command_scripts table
ALTER TABLE command_scripts
ADD COLUMN language TEXT NOT NULL DEFAULT 'shell';