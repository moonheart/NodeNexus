import apiClient from './apiClient.ts'; // Assuming you have an apiClient for making requests
import type { Vps } from '../types'; // Assuming Vps type is defined in src/types.ts

export interface CreateVpsPayload {
  name: string;
}

// The backend is expected to return the full Vps object upon creation,
// including id and agent_secret.
export const createVps = async (payload: CreateVpsPayload): Promise<Vps> => {
  try {
    const response = await apiClient.post<Vps>('/api/vps', payload); // Changed path
    return response.data;
  } catch (error) {
    // Handle or throw error as appropriate for your error handling strategy
    console.error('Error creating VPS:', error);
    throw error;
  }
};

/**
 * Fetches the list of VPS for the authenticated user.
 */
export const getVpsList = async (): Promise<Vps[]> => {
  try {
    const response = await apiClient.get<Vps[]>('/api/vps'); // GET request to the same endpoint
    return response.data;
  } catch (error) {
    console.error('Error fetching VPS list:', error);
    throw error;
  }
};

// You might want to add other VPS related API calls here in the future,
// e.g., getVpsById, deleteVps, etc.