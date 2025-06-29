-- Up Migration
CREATE TABLE themes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL CHECK (type IN ('light', 'dark')),
    is_official BOOLEAN NOT NULL DEFAULT FALSE,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add theme settings to users table
ALTER TABLE users
ADD COLUMN theme_mode VARCHAR(50) NOT NULL DEFAULT 'system' CHECK (theme_mode IN ('light', 'dark', 'system')),
ADD COLUMN active_light_theme_id UUID REFERENCES themes(id) ON DELETE SET NULL,
ADD COLUMN active_dark_theme_id UUID REFERENCES themes(id) ON DELETE SET NULL;

-- Create indexes for performance
CREATE INDEX idx_themes_user_id ON themes(user_id);
CREATE INDEX idx_themes_is_official ON themes(is_official);

-- Down Migration
DROP TABLE IF EXISTS themes;

ALTER TABLE users
DROP COLUMN IF EXISTS theme_mode,
DROP COLUMN IF EXISTS active_light_theme_id,
DROP COLUMN IF EXISTS active_dark_theme_id;