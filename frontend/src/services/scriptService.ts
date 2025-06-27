import { useAuthStore } from '../store/authStore';

// This should match the backend's command_script::Model
export interface CommandScript {
    id: number;
    user_id: number;
    name: string;
    description: string | null;
    language: 'shell' | 'powershell';
    script_content: string;
    working_directory: string;
    created_at: string;
    updated_at: string;
}

// This should match the backend's ScriptPayload
export type ScriptPayload = Omit<CommandScript, 'id' | 'user_id' | 'created_at' | 'updated_at'>;

const getHeaders = () => {
    const { token } = useAuthStore.getState();
    return {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
    };
};

const handleResponse = async (response: Response) => {
    if (!response.ok) {
        const errorData = await response.json().catch(() => ({ message: 'An unknown error occurred.' }));
        throw new Error(errorData.error || 'Failed to perform operation.');
    }
    if (response.status === 204) { // No Content
        return;
    }
    return response.json();
};

export const scriptService = {
    getScripts: async (): Promise<CommandScript[]> => {
        const response = await fetch('/api/command-scripts', {
            headers: getHeaders(),
        });
        return handleResponse(response);
    },

    createScript: async (data: ScriptPayload): Promise<CommandScript> => {
        const response = await fetch('/api/command-scripts', {
            method: 'POST',
            headers: getHeaders(),
            body: JSON.stringify(data),
        });
        return handleResponse(response);
    },

    updateScript: async (id: number, data: ScriptPayload): Promise<CommandScript> => {
        const response = await fetch(`/api/command-scripts/${id}`, {
            method: 'PUT',
            headers: getHeaders(),
            body: JSON.stringify(data),
        });
        return handleResponse(response);
    },

    deleteScript: async (id: number): Promise<void> => {
        const response = await fetch(`/api/command-scripts/${id}`, {
            method: 'DELETE',
            headers: getHeaders(),
        });
        await handleResponse(response);
    },
};