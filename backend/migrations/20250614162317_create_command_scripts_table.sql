-- Migration for creating the command_scripts table
CREATE TABLE command_scripts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    script_content TEXT NOT NULL,
    working_directory VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add a unique constraint on user_id and name to prevent duplicate script names for the same user
ALTER TABLE command_scripts ADD CONSTRAINT uq_user_script_name UNIQUE (user_id, name);

-- Add an index for faster lookups by user_id
CREATE INDEX idx_command_scripts_user_id ON command_scripts(user_id);
