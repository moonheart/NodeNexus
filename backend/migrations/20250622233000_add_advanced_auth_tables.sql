-- MIGRATION: Add advanced authentication features

-- Step 1: Modify the existing 'users' table
ALTER TABLE "users"
    ALTER COLUMN "password_hash" DROP NOT NULL,
    ADD COLUMN "role" VARCHAR NOT NULL DEFAULT 'user',
    ADD COLUMN "password_login_disabled" BOOLEAN NOT NULL DEFAULT FALSE;

-- Step 2: Create the 'oauth2_providers' table for dynamic configuration
CREATE TABLE "oauth2_providers" (
    "id" SERIAL PRIMARY KEY,
    "provider_name" VARCHAR(255) UNIQUE NOT NULL,
    "client_id" VARCHAR(255) NOT NULL,
    "client_secret" TEXT NOT NULL, -- Encrypted value
    "auth_url" VARCHAR(255) NOT NULL,
    "token_url" VARCHAR(255) NOT NULL,
    "user_info_url" VARCHAR(255) NOT NULL,
    "scopes" TEXT,
    "user_info_mapping" JSONB,
    "enabled" BOOLEAN NOT NULL DEFAULT TRUE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT (now()),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT (now())
);

-- Step 3: Create the 'user_identity_providers' table to link users to providers
CREATE TABLE "user_identity_providers" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
    "provider_name" VARCHAR(255) NOT NULL,
    "provider_user_id" VARCHAR(255) NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT (now()),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT (now()),
    UNIQUE("provider_name", "provider_user_id")
);

-- Add indexes for better performance
CREATE INDEX "idx_oauth2_providers_provider_name" ON "oauth2_providers"("provider_name");
CREATE INDEX "idx_user_identity_providers_user_id" ON "user_identity_providers"("user_id");