import apiClient from './apiClient';
import type { CommandScript } from '../types';

// This should match the backend's ScriptPayload
export type ScriptPayload = Omit<CommandScript, 'id' | 'user_id' | 'created_at' | 'updated_at'>;

export const scriptService = {
    getScripts: async (): Promise<CommandScript[]> => {
        const response = await apiClient.get('/command-scripts');
        return response.data;
    },

    createScript: async (data: ScriptPayload): Promise<CommandScript> => {
        const response = await apiClient.post('/command-scripts', data);
        return response.data;
    },

    updateScript: async (id: number, data: ScriptPayload): Promise<CommandScript> => {
        const response = await apiClient.put(`/command-scripts/${id}`, data);
        return response.data;
    },

    deleteScript: async (id: number): Promise<void> => {
        await apiClient.delete(`/command-scripts/${id}`);
    },
};