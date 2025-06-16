-- MIGRATION_BASE_NAME: add_agent_version_to_vps
-- MIGRATION_CREATED_AT: 2025-06-17 00:33:00

-- UP
ALTER TABLE "vps" ADD COLUMN "agent_version" VARCHAR(255);

-- DOWN
ALTER TABLE "vps" DROP COLUMN "agent_version";