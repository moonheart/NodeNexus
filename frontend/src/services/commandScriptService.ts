import apiClient from './apiClient';
import type { CommandScript } from '../types';

export const createCommandScript = async (
    name: string,
    description: string | undefined,
    script_content: string,
    working_directory: string
): Promise<CommandScript> => {
    const response = await apiClient.post('/command-scripts', {
        name,
        description,
        script_content,
        working_directory,
    });
    return response.data;
};

export const getCommandScripts = async (): Promise<CommandScript[]> => {
    const response = await apiClient.get('/command-scripts');
    return response.data;
};

export const updateCommandScript = async (
    id: number,
    name: string,
    description: string | undefined,
    script_content: string,
    working_directory: string
): Promise<CommandScript> => {
    const response = await apiClient.put(`/command-scripts/${id}`, {
        name,
        description,
        script_content,
        working_directory,
    });
    return response.data;
};

export const deleteCommandScript = async (id: number): Promise<void> => {
    await apiClient.delete(`/command-scripts/${id}`);
};