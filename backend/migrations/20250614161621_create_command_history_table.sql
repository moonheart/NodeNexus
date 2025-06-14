-- Add migration script here
CREATE TABLE command_history (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    command_text TEXT NOT NULL,
    working_directory VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, command_text, working_directory) -- Prevent exact duplicates for the same user
);

-- Optional: Add an index for faster lookups
CREATE INDEX idx_command_history_user_id ON command_history(user_id);
