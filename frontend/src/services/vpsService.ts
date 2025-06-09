import apiClient from './apiClient.ts'; // Assuming you have an apiClient for making requests
// VpsListItemResponse is the type returned by the backend for list and detail views now
import type { Vps, VpsListItemResponse } from '../types';

export interface CreateVpsPayload {
  name: string;
  // Optional traffic monitoring config fields
  traffic_limit_bytes?: number | null;
  traffic_billing_rule?: string | null;
  traffic_reset_config_type?: string | null;
  traffic_reset_config_value?: string | null;
  next_traffic_reset_at?: string | null; // ISO string for DateTime<Utc>
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
  // Traffic monitoring config fields
  traffic_limit_bytes?: number | null;
  traffic_billing_rule?: string | null;
  traffic_reset_config_type?: string | null;
  traffic_reset_config_value?: string | null;
  next_traffic_reset_at?: string | null; // ISO string for DateTime<Utc>

  // Renewal Info Fields (matching backend UpdateVpsRequest)
  renewalCycle?: string | null;
  renewalCycleCustomDays?: number | null;
  renewalPrice?: number | null;
  renewalCurrency?: string | null;
  nextRenewalDate?: string | null; // ISO string for DateTime<Utc>
  lastRenewalDate?: string | null; // ISO string for DateTime<Utc>
  serviceStartDate?: string | null; // ISO string for DateTime<Utc>
  paymentMethod?: string | null;
  autoRenewEnabled?: boolean | null;
  renewalNotes?: string | null;
  // reminderActive is managed by backend
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

/**
 * Dismisses the active renewal reminder for a specific VPS.
 */
export const dismissVpsRenewalReminder = async (vpsId: number): Promise<void> => {
  try {
    await apiClient.post(`/api/vps/${vpsId}/renewal/dismiss-reminder`);
  } catch (error) {
    console.error(`Error dismissing renewal reminder for VPS ID ${vpsId}:`, error);
    throw error;
  }
};

// You might want to add other VPS related API calls here in the future,
// e.g., deleteVps, etc.