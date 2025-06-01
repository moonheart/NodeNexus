// 类型定义需要与 backend/src/http_server/auth_logic.rs 中的结构体对齐
// 为了减少跨模块直接导入的复杂性，我们在这里重新定义或从共享类型文件导入（如果未来创建的话）

export interface RegisterRequest {
    username: string;
    email: string;
    password: string;
}

export interface UserResponse {
    id: number;
    username: string;
    email: string;
}

export interface LoginRequest {
    email: string; // 在后端，这可以是 username 或 email
    password: string;
}

export interface LoginResponse {
    token: string;
    user_id: number;
    username: string;
    email: string;
}

const ENV_API_ROOT = import.meta.env.VITE_API_BASE_URL;
let AUTH_API_ENDPOINT_BASE: string;

if (ENV_API_ROOT) {
    // If VITE_API_BASE_URL is set, append /api/auth to it
    AUTH_API_ENDPOINT_BASE = `${ENV_API_ROOT.replace(/\/$/, '')}/api/auth`;
} else {
    // Otherwise, default to relative /api/auth
    AUTH_API_ENDPOINT_BASE = '/api/auth';
}

export const registerUser = async (data: RegisterRequest): Promise<UserResponse> => {
    console.log('authService.ts: registerUser called with', data, 'to endpoint', `${AUTH_API_ENDPOINT_BASE}/register`);
    const response = await fetch(`${AUTH_API_ENDPOINT_BASE}/register`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
    });
    if (!response.ok) {
        // Try to parse error message from backend, otherwise use a generic one
        let errorMsg = '注册失败';
        try {
            const errorData = await response.json();
            if (errorData && errorData.error) { // Check for .error field
                errorMsg = errorData.error;
            } else if (errorData && errorData.message) { // Fallback to .message
                errorMsg = errorData.message;
            } else if (typeof errorData === 'string') {
                errorMsg = errorData;
            }
        } catch (_e) { // eslint-disable-line @typescript-eslint/no-unused-vars
            // If parsing error fails, use the status text or generic message
            errorMsg = response.statusText || errorMsg;
        }
        throw new Error(errorMsg);
    }
    return response.json();
};

export const loginUser = async (data: LoginRequest): Promise<LoginResponse> => {
    console.log('authService.ts: loginUser called with', data, 'to endpoint', `${AUTH_API_ENDPOINT_BASE}/login`);
    const response = await fetch(`${AUTH_API_ENDPOINT_BASE}/login`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
    });
    if (!response.ok) {
        // Try to parse error message from backend, otherwise use a generic one
        let errorMsg = '登录失败';
        try {
            const errorData = await response.json();
            if (errorData && errorData.error) { // Check for .error field
                errorMsg = errorData.error;
            } else if (errorData && errorData.message) { // Fallback to .message
                errorMsg = errorData.message;
            } else if (typeof errorData === 'string') {
                errorMsg = errorData;
            }
        } catch (_e) { // eslint-disable-line @typescript-eslint/no-unused-vars
            // If parsing error fails, use the status text or generic message
            errorMsg = response.statusText || errorMsg;
        }
        throw new Error(errorMsg);
    }
    return response.json();
};