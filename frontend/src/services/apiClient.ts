import axios, { AxiosError, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';
import { useAuthStore } from '../store/authStore'; // Assuming authStore is where token is managed

const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api', // Use environment variable or default
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request interceptor to add JWT token to headers
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const token = useAuthStore.getState().token; // Get token from Zustand store
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error: AxiosError) => {
    return Promise.reject(error);
  }
);

// --- Development Settings ---
// Set to true to simulate a 500ms network delay on all API requests.
const SIMULATE_DELAY = false;
// --- End of Development Settings ---

const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// Optional: Response interceptor for global error handling (e.g., 401 unauthorized)
apiClient.interceptors.response.use(
  async (response: AxiosResponse) => {
    if (SIMULATE_DELAY) {
      await delay(500);
    }
    return response;
  },
  async (error: AxiosError) => {
    if (SIMULATE_DELAY) {
      await delay(500);
    }
    if (error.response && error.response.status === 401) { // Check if error.response exists
      // Handle 401, e.g., redirect to login, clear token
      useAuthStore.getState().logout(); // Example: logout user
      window.location.href = '/login'; // Or redirect
      console.error('Unauthorized, logging out.');
    }
    return Promise.reject(error);
  }
);

export { axios };
export default apiClient;