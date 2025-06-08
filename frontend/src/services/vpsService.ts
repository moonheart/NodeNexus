import apiClient from './apiClient.ts'; // Assuming you have an apiClient for making requests
// VpsListItemResponse is the type returned by the backend for list and detail views now
import type { Vps, VpsListItemResponse } from '../types';

export interface CreateVpsPayload {
  name: string;
}

// The backend is expected to return the full Vps object upon creation,
// including id and agent_secret.
// For create, the backend still returns the basic Vps model.
export const createVps = async (payload: CreateVpsPayload): Promise<Vps> => {
  try {
    const response = await apiClient.post<Vps>('/api/vps', payload);
    return response.data;
  } catch (error) {
    console.error('Error creating VPS:', error);
    throw error;
  }
};


/**
 * Fetches the details for a single VPS.
 */
export const getVpsDetail = async (vpsId: string): Promise<VpsListItemResponse> => {
  try {
    const response = await apiClient.get<VpsListItemResponse>(`/api/vps/${vpsId}`);
    return response.data;
  } catch (error) {
    console.error(`Error fetching VPS detail for ID ${vpsId}:`, error);
    throw error;
  }
};


export interface UpdateVpsPayload {
  name?: string;
  group?: string;
  tag_ids?: number[];
}

/**
 * Updates a VPS's details.
 */
export const updateVps = async (vpsId: number, payload: UpdateVpsPayload): Promise<void> => {
  try {
    await apiClient.put(`/api/vps/${vpsId}`, payload);
  } catch (error) {
    console.error(`Error updating VPS with ID ${vpsId}:`, error);
    throw error;
  }
};

/**
 * Fetches all VPS list items.
 * Useful for populating dropdowns or lists where full details are not immediately needed per item.
 */
export const getAllVpsListItems = async (): Promise<VpsListItemResponse[]> => {
  try {
    const response = await apiClient.get<VpsListItemResponse[]>('/api/vps'); // Assuming '/api/vps' endpoint returns the list
    return response.data;
  } catch (error) {
    console.error('Error fetching all VPS list items:', error);
    throw error;
  }
};


// You might want to add other VPS related API calls here in the future,
// e.g., deleteVps, etc.