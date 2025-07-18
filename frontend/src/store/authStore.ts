import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { loginUser, registerUser, getMe } from '../services/authService';
import type { LoginRequest, RegisterRequest, UserResponse, LoginResponse } from '../services/authService';
import websocketService from '../services/websocketService';

interface AuthState {
    isAuthenticated: boolean;
    user: UserResponse | null;
    token: string | null;
    isLoading: boolean;
    error: string | null;
    login: (credentials: LoginRequest) => Promise<void>;
    register: (userData: RegisterRequest) => Promise<void>;
    logout: () => void;
    setToken: (token: string | null) => void;
    setUser: (user: UserResponse | null) => void;
    fetchUser: () => Promise<void>;
    clearAuthError: () => void;
}

export const useAuthStore = create<AuthState>()(
    persist(
        (set) => ({
            isAuthenticated: false,
            user: null,
            token: null,
            isLoading: false,
            error: null,

            login: async (credentials: LoginRequest) => {
                set({ isLoading: true, error: null });
                try {
                    const response: LoginResponse = await loginUser(credentials);
                    set({
                        isAuthenticated: true,
                        user: { id: response.user_id, username: response.username },
                        token: response.token,
                        isLoading: false,
                        error: null,
                    });
                    // Disconnect any existing WS connection and reconnect with the new token
                    websocketService.disconnect();
                    websocketService.connect(response.token);
                    // console.log("Login successful, token:", response.token);
                } catch (err: unknown) {
                    const errorMessage = err instanceof Error ? err.message : '登录时发生未知错误';
                    set({ isAuthenticated: false, user: null, token: null, isLoading: false, error: errorMessage });
                    // console.error("Login failed:", errorMessage);
                    throw err; // Re-throw to allow components to handle it if needed
                }
            },

            register: async (userData: RegisterRequest) => {
                set({ isLoading: true, error: null });
                try {
                    // UserResponse is returned by registerUser, no token here
                    await registerUser(userData);
                    set({ isLoading: false, error: null }); // User is not logged in after registration
                    // console.log("Registration successful for:", userData.email);
                } catch (err: unknown) {
                    const errorMessage = err instanceof Error ? err.message : '注册时发生未知错误';
                    set({ isLoading: false, error: errorMessage });
                    // console.error("Registration failed:", errorMessage);
                    throw err; // Re-throw
                }
            },

            logout: () => {
                set({ isAuthenticated: false, user: null, token: null, error: null });
                // Disconnect the authenticated WS connection and reconnect to the public endpoint
                websocketService.disconnect();
                websocketService.connect(); // Reconnect without a token
                // console.log("User logged out");
            },

            setToken: (token: string | null) => {
                set({
                    token: token,
                });
            },

            setUser: (user: UserResponse | null) => {
                set({ user });
            },

            clearAuthError: () => {
                set({ error: null });
            },

            fetchUser: async () => {
                set({ isLoading: true, error: null });
                try {
                    const user = await getMe();
                    set({
                        isAuthenticated: true,
                        user,
                        isLoading: false,
                        error: null,
                    });
                } catch (err: unknown) {
                    const errorMessage = err instanceof Error ? err.message : 'Session expired or invalid. Please log in again.';
                    set({
                        isAuthenticated: false,
                        user: null,
                        token: null, // Also clear token if fetch fails
                        isLoading: false,
                        error: errorMessage,
                    });
                    // Disconnect WebSocket if user fetch fails
                    websocketService.disconnect();
                }
            },
        }),
        {
            name: 'auth-storage', // name of the item in the storage (must be unique)
            storage: createJSONStorage(() => localStorage), // (optional) by default, 'localStorage' is used
            partialize: (state) => ({ token: state.token, user: state.user, isAuthenticated: state.isAuthenticated }), // Persist only token, user and isAuthenticated
        }
    )
);

// Initialize auth state from storage on app load
// This helps to keep the user logged in across page refreshes if token is valid
const initialToken = (JSON.parse(localStorage.getItem('auth-storage') || '{}').state as AuthState)?.token;
const initialUser = (JSON.parse(localStorage.getItem('auth-storage') || '{}').state as AuthState)?.user;
const initialIsAuthenticated = (JSON.parse(localStorage.getItem('auth-storage') || '{}').state as AuthState)?.isAuthenticated;

if (initialToken && initialUser && initialIsAuthenticated) {
    useAuthStore.setState({ token: initialToken, user: initialUser, isAuthenticated: initialIsAuthenticated });
    // console.log("Auth state initialized from localStorage:", { token: initialToken, user: initialUser, isAuthenticated: initialIsAuthenticated });
}