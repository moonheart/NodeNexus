// 类型定义需要与 backend/src/http_server/auth_logic.rs 中的结构体对齐
// 为了减少跨模块直接导入的复杂性，我们在这里重新定义或从共享类型文件导入（如果未来创建的话）

import apiClient, { axios } from "./apiClient";

export interface RegisterRequest {
    username: string;
    password: string;
}

export interface UserResponse {
    id: number;
    username: string;
}

export interface LoginRequest {
    username: string;
    password: string;
}

export interface LoginResponse {
    token: string;
    user_id: number;
    username: string;
}

// 不再需要手动构建 URL，apiClient 会处理 baseURL
// const ENV_API_ROOT = import.meta.env.VITE_API_BASE_URL;
// let AUTH_API_ENDPOINT_BASE: string;
//
// if (ENV_API_ROOT) {
//     // If VITE_API_BASE_URL is set, append /api/auth to it
//     AUTH_API_ENDPOINT_BASE = `${ENV_API_ROOT.replace(/\/$/, '')}/api/auth`;
// } else {
//     // Otherwise, default to relative /api/auth
//     AUTH_API_ENDPOINT_BASE = '/api/auth';
// }

export const registerUser = async (data: RegisterRequest): Promise<UserResponse> => {
    console.log('authService.ts: registerUser called with', data);
    try {
        const response = await apiClient.post<UserResponse>('/auth/register', data);
        return response.data;
    } catch (error: unknown) {
        console.error('Registration failed:', error);
        // 尝试从 axios 错误中提取后端返回的错误信息
        let errorMsg = '注册失败';
        if (axios.isAxiosError(error) && error.response?.data) {
            const errorData = error.response.data;
            if (typeof errorData === 'string') {
                errorMsg = errorData;
            } else if (errorData && typeof errorData === 'object') {
                if ('error' in errorData && typeof errorData.error === 'string') {
                    errorMsg = errorData.error;
                } else if ('message' in errorData && typeof errorData.message === 'string') {
                    errorMsg = errorData.message; // 备选 message 字段
                }
            }
        } else if (error instanceof Error) {
            errorMsg = error.message; // 其他类型的错误
        }
        throw new Error(errorMsg);
    }
};

export const loginUser = async (data: LoginRequest): Promise<LoginResponse> => {
    console.log('authService.ts: loginUser called with', data);
    try {
        const response = await apiClient.post<LoginResponse>('/auth/login', data);
        return response.data;
    } catch (error: unknown) {
        console.error('Login failed:', error);
        // 尝试从 axios 错误中提取后端返回的错误信息
        let errorMsg = '登录失败';
        if (axios.isAxiosError(error) && error.response?.data) {
            const errorData = error.response.data;
             if (typeof errorData === 'string') {
                errorMsg = errorData;
            } else if (errorData && typeof errorData === 'object') {
                 if ('error' in errorData && typeof errorData.error === 'string') {
                    errorMsg = errorData.error;
                } else if ('message' in errorData && typeof errorData.message === 'string') {
                    errorMsg = errorData.message; // 备选 message 字段
                }
            }
        } else if (error instanceof Error) {
            errorMsg = error.message; // 其他类型的错误
        }
        throw new Error(errorMsg);
    }
};

export interface AuthProvider {
    name: string;
    iconUrl: string | undefined;
}

export const getAuthProviders = async (): Promise<AuthProvider[]> => {
    const response = await apiClient.get<AuthProvider[]>('/auth/providers');
    return response.data;
};

export const getMe = async (): Promise<UserResponse> => {
    try {
        const response = await apiClient.get<UserResponse>('/auth/me');
        return response.data;
    } catch (error) {
        console.error('Failed to fetch user data:', error);
        throw error;
    }
};