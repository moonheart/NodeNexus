import apiClient from './apiClient';
import type { AgentConfig } from '../types'; // We will need to define this type

export const getGlobalConfig = async (): Promise<AgentConfig> => {
    const response = await apiClient.get<AgentConfig>('/settings/agent-config');
    return response.data;
};

export const updateGlobalConfig = async (config: AgentConfig): Promise<void> => {
    await apiClient.put('/settings/agent-config', config);
};

export const retryConfigPush = async (vpsId: number): Promise<void> => {
    await apiClient.post(`/vps/${vpsId}/retry-config`);
};

export const pushConfig = async (vpsId: number): Promise<void> => {
    await apiClient.post(`/vps/${vpsId}/push-config`);
};

export const previewConfig = async (vpsId: number): Promise<AgentConfig> => {
    const response = await apiClient.get<AgentConfig>(`/vps/${vpsId}/config-preview`);
    return response.data;
};