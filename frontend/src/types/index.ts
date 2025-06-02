// This file can be used to store all common TypeScript types for the frontend.

/**
 * Represents a Virtual Private Server (VPS) as defined by the backend.
 * This should match the structure of the `Vps` model in `backend/src/db/models.rs`.
 */
export interface Vps {
  id: number;
  user_id: number;
  name: string;
  ip_address: string | null;
  os_type: string | null;
  agent_secret: string;
  status: string;
  metadata: Record<string, unknown> | null; // Can be refined if the structure of metadata is known
  created_at: string; // Represents a `DateTime<Utc>` string, e.g., "2025-06-02T12:34:56.789Z"
  updated_at: string; // Represents a `DateTime<Utc>` string
}