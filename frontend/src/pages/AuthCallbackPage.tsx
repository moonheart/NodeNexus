import React, { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import websocketService from '../services/websocketService';

const AuthCallbackPage: React.FC = () => {
    const navigate = useNavigate();
    const { fetchUser, setToken } = useAuthStore();

    useEffect(() => {
        const handleAuth = async () => {
            try {
                // Get token from URL query params
                const params = new URLSearchParams(window.location.search);
                const token = params.get('token');
                if (token) {
                    setToken(token);
                }
                
                await fetchUser();
                websocketService.disconnect();
                websocketService.connect(token);
                // Redirect to the home page or a desired page after login
                navigate('/');
            } catch (error) {
                console.error('Login failed:', error);
                // Redirect to a login page or show an error message
                navigate('/login');
            }
        };

        handleAuth();
    }, [fetchUser, navigate]);

    return (
        <div className="flex items-center justify-center h-screen">
            <div className="text-center">
                <p className="text-lg font-semibold">Finalizing login...</p>
                <p className="text-gray-500">Please wait while we securely log you in.</p>
            </div>
        </div>
    );
};

export default AuthCallbackPage;