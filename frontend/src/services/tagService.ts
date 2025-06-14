import apiClient from './apiClient';
import type { Tag, CreateTagPayload, UpdateTagPayload } from '../types';

// Type for bulk updating tags on VPSs
export interface BulkUpdateVpsTagsPayload {
    vpsIds: number[];
    addTagIds: number[];
    removeTagIds: number[];
}

/**
 * Fetches all tags for the current user, including their usage count.
 * Corresponds to GET /api/tags
 */
export const getTags = async (): Promise<Tag[]> => {
  const response = await apiClient.get<Tag[]>('/tags');
  return response.data;
};

/**
 * Creates a new tag.
 * Corresponds to POST /api/tags
 */
export const createTag = async (payload: CreateTagPayload): Promise<Tag> => {
  const response = await apiClient.post<Tag>('/tags', payload);
  return response.data;
};

/**
 * Updates an existing tag.
 * Corresponds to PUT /api/tags/:tagId
 */
export const updateTag = async (tagId: number, payload: UpdateTagPayload): Promise<void> => {
  await apiClient.put(`/tags/${tagId}`, payload);
};

/**
 * Deletes a tag.
 * Corresponds to DELETE /api/tags/:tagId
 */
export const deleteTag = async (tagId: number): Promise<void> => {
  await apiClient.delete(`/tags/${tagId}`);
};

/**
 * Adds a tag to a specific VPS.
 * Corresponds to POST /api/vps/:vpsId/tags
 */
export const addTagToVps = async (vpsId: number, tagId: number): Promise<void> => {
  await apiClient.post(`/vps/${vpsId}/tags`, { tag_id: tagId });
};

/**
 * Removes a tag from a specific VPS.
 * Corresponds to DELETE /api/vps/:vpsId/tags/:tagId
 */
export const removeTagFromVps = async (vpsId: number, tagId: number): Promise<void> => {
  await apiClient.delete(`/vps/${vpsId}/tags/${tagId}`);
};

/**
 * Bulk adds/removes tags for multiple VPSs.
 * Corresponds to POST /api/vps/bulk-actions
 */
export const bulkUpdateVpsTags = async (payload: BulkUpdateVpsTagsPayload): Promise<void> => {
    await apiClient.post('/vps/bulk-actions', payload);
};