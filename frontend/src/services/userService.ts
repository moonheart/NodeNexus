import { useAuthStore } from '../store/authStore';

const getAuthHeaders = () => {
    const token = useAuthStore.getState().token;
    return {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
    };
};

// A helper function to handle API responses
async function handleResponse<T>(response: Response): Promise<T> {
    if (!response.ok) {
        const errorData = await response.json().catch(() => ({ message: 'An unknown error occurred.' }));
        throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
    }
    return response.json();
}

export interface User {
    id: number;
    username: string;
}

export interface ConnectedAccount {
    provider_name: string;
    provider_user_id: string;
}

export interface OAuthProvider {
    name: string;
    iconUrl: string | undefined;
}

export const updateUsername = async (username: string): Promise<User> => {
    const response = await fetch('/api/user/username', {
        method: 'PUT',
        headers: getAuthHeaders(),
        body: JSON.stringify({ username }),
    });
    return handleResponse<User>(response);
};

export const updatePassword = async (passwordData: object): Promise<{ message: string }> => {
    const response = await fetch('/api/user/password', {
        method: 'PUT',
        headers: getAuthHeaders(),
        body: JSON.stringify(passwordData),
    });
    return handleResponse<{ message: string }>(response);
};

export const getConnectedAccounts = async (): Promise<ConnectedAccount[]> => {
    const response = await fetch('/api/user/connected-accounts', {
        headers: getAuthHeaders(),
    });
    return handleResponse<ConnectedAccount[]>(response);
};

export const unlinkProvider = async (providerName: string) => {
    const response = await fetch(`/api/user/connected-accounts/${providerName}`, {
        method: 'DELETE',
        headers: getAuthHeaders(),
    });
    return handleResponse(response);
};
export const getAvailableProviders = async (): Promise<OAuthProvider[]> => {
    const response = await fetch('/api/auth/providers', {
        headers: getAuthHeaders(),
    });
    return handleResponse<OAuthProvider[]>(response);
};

export const updateUserLanguage = async (language: string): Promise<{ message: string }> => {
    const response = await fetch('/api/user/preference', {
        method: 'PUT',
        headers: getAuthHeaders(),
        body: JSON.stringify({ language }),
    });
    return handleResponse<{ message: string }>(response);
};