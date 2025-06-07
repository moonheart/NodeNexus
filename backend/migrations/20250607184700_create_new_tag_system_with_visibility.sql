-- Migration to create the new tag system with visibility control

-- 1. Create the 'tags' table for storing tag details.
-- Each tag is owned by a user and has properties like color, icon, and an optional URL.
-- A new 'is_visible' column is added to control frontend visibility.
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#ffffff', -- Hex color code, e.g., #RRGGBB
    icon VARCHAR(255), -- Name of the icon from a library like Lucide
    url VARCHAR(2048), -- Optional URL associated with the tag
    is_visible BOOLEAN NOT NULL DEFAULT TRUE, -- Controls visibility in the frontend
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, name) -- A user cannot have two tags with the same name
);

-- 2. Create the 'vps_tags' join table.
-- This table establishes a many-to-many relationship between VPS and tags.
CREATE TABLE vps_tags (
    vps_id INTEGER NOT NULL REFERENCES vps(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (vps_id, tag_id)
);

-- 3. Remove the old 'tags' column from the 'vps' table.
-- The 'group' column is kept as a separate feature as requested.
ALTER TABLE vps DROP COLUMN IF EXISTS tags;