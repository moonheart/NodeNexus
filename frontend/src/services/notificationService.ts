import apiClient from './apiClient';
import type { ChannelTemplate, ChannelResponse, CreateChannelRequest, UpdateChannelRequest } from '../types';

/**
 * Fetches all available notification channel templates from the backend.
 * These templates are used to dynamically generate configuration forms.
 */
export const getChannelTemplates = async (): Promise<ChannelTemplate[]> => {
    const response = await apiClient.get<ChannelTemplate[]>('/notifications/channels/templates');
    return response.data;
};

/**
 * Fetches all of a user's configured notification channels.
 */
export const getAllChannels = async (): Promise<ChannelResponse[]> => {
    const response = await apiClient.get<ChannelResponse[]>('/notifications/channels');
    return response.data;
};

/**
 * Creates a new notification channel.
 * @param channelData - The data for the new channel.
 */
export const createChannel = async (channelData: CreateChannelRequest): Promise<ChannelResponse> => {
    const response = await apiClient.post<ChannelResponse>('/notifications/channels', channelData);
    return response.data;
};

/**
 * Updates an existing notification channel.
 * @param id - The ID of the channel to update.
 * @param channelData - The data to update.
 */
export const updateChannel = async (id: number, channelData: UpdateChannelRequest): Promise<ChannelResponse> => {
    const response = await apiClient.put<ChannelResponse>(`/notifications/channels/${id}`, channelData);
    return response.data;
};

/**
 * Deletes a notification channel.
 * @param id - The ID of the channel to delete.
 */
export const deleteChannel = async (id: number): Promise<void> => {
    await apiClient.delete(`/notifications/channels/${id}`);
};

/**
 * Sends a test message to a specific notification channel.
 * @param id - The ID of the channel to test.
 * @param message - An optional custom message for the test.
 */
export const testChannel = async (id: number, message?: string): Promise<{ message: string }> => {
    const response = await apiClient.post<{ message: string }>(`/notifications/channels/${id}/test`, { message });
    return response.data;
};