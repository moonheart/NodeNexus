import apiClient from './apiClient';

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
    const response = await apiClient.put<User>('/user/username', { username });
    return response.data;
};

export const updatePassword = async (passwordData: object): Promise<{ message: string }> => {
    const response = await apiClient.put<{ message: string }>('/user/password', passwordData);
    return response.data;
};

export const getConnectedAccounts = async (): Promise<ConnectedAccount[]> => {
    const response = await apiClient.get<ConnectedAccount[]>('/user/connected-accounts');
    return response.data;
};

export const unlinkProvider = async (providerName: string) => {
    const response = await apiClient.delete(`/user/connected-accounts/${providerName}`);
    return response.data;
};
export const getAvailableProviders = async (): Promise<OAuthProvider[]> => {
    const response = await apiClient.get<OAuthProvider[]>('/auth/providers');
    return response.data;
};

export const updateUserLanguage = async (language: string): Promise<{ message: string }> => {
    const response = await apiClient.put<{ message: string }>('/user/preference', { language });
    return response.data;
};