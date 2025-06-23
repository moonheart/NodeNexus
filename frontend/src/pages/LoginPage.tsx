import React, { useState, useEffect } from 'react';
import { useNavigate, Link as RouterLink } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import { getAuthProviders } from '../services/authService';
import type { LoginRequest, AuthProvider } from '../services/authService';

const LoginPage: React.FC = () => {
    const [username, setUsername] = useState('');
    const [password, setPassword] = useState('');
    const [providers, setProviders] = useState<AuthProvider[]>([]);
    const { login, isLoading, error, isAuthenticated, clearAuthError } = useAuthStore();
    const navigate = useNavigate();

    useEffect(() => {
        if (isAuthenticated) {
            navigate('/', { replace: true });
        }
    }, [isAuthenticated, navigate]);

    useEffect(() => {
        const fetchProviders = async () => {
            try {
                const availableProviders = await getAuthProviders();
                setProviders(availableProviders);
            } catch (err) {
                console.error("Failed to fetch auth providers:", err);
            }
        };
        fetchProviders();
    }, []);

    const handleSubmit = async (event: React.FormEvent) => {
        event.preventDefault();
        clearAuthError();
        const credentials: LoginRequest = { username, password };
        try {
            await login(credentials);
        } catch (err) {
            console.error('Login failed on page:', err);
        }
    };

    return (
        <div className="flex items-center justify-center min-h-screen bg-slate-50">
            <div className="w-full max-w-md p-8 space-y-6 bg-white rounded-lg shadow-md">
                <h1 className="text-2xl font-bold text-center text-slate-900">
                    登录您的账户
                </h1>
                <form className="space-y-6" onSubmit={handleSubmit}>
                    <div>
                        <label htmlFor="username" className="block text-sm font-medium text-slate-700">
                            用户名
                        </label>
                        <input
                            id="username"
                            name="username"
                            type="text"
                            autoComplete="username"
                            required
                            className="mt-1 block w-full px-3 py-2 bg-white border border-slate-300 rounded-md shadow-sm placeholder-slate-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                        />
                    </div>
                    <div>
                        <label htmlFor="password" className="block text-sm font-medium text-slate-700">
                            密码
                        </label>
                        <input
                            id="password"
                            name="password"
                            type="password"
                            autoComplete="current-password"
                            required
                            className="mt-1 block w-full px-3 py-2 bg-white border border-slate-300 rounded-md shadow-sm placeholder-slate-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                        />
                    </div>

                    {error && (
                        <div className="p-3 text-sm text-red-700 bg-red-100 rounded-md">
                            {error}
                        </div>
                    )}

                    <div>
                        <button
                            type="submit"
                            disabled={isLoading}
                            className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-400 disabled:cursor-not-allowed"
                        >
                            {isLoading ? '登录中...' : '登录'}
                        </button>
                    </div>
                </form>

                <div className="relative my-4">
                   <div className="absolute inset-0 flex items-center">
                       <div className="w-full border-t border-slate-300" />
                   </div>
                   <div className="relative flex justify-center text-sm">
                       <span className="px-2 bg-white text-slate-500">
                           或者通过以下方式继续
                       </span>
                   </div>
               </div>

              <div>
                  {providers.map((provider) => (
                      <button
                          key={provider.provider_name}
                          type="button"
                          onClick={() => window.location.href = `/api/auth/${provider.provider_name}/login`}
                          className="w-full flex justify-center items-center py-2 px-4 border border-slate-300 rounded-md shadow-sm text-sm font-medium text-slate-700 bg-white hover:bg-slate-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
                      >
                          {/* You can add specific icons based on provider_name */}
                          <svg className="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                              <path fillRule="evenodd" d="M10 0C4.477 0 0 4.477 0 10c0 4.418 2.865 8.166 6.839 9.49.5.092.682-.217.682-.482 0-.237-.009-.868-.014-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.031-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0110 4.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.378.203 2.398.1 2.651.64.7 1.03 1.595 1.03 2.688 0 3.848-2.338 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.001 10.001 0 0020 10c0-5.523-4.477-10-10-10z" clipRule="evenodd" />
                          </svg>
                          使用 {provider.provider_name.charAt(0).toUpperCase() + provider.provider_name.slice(1)} 登录
                      </button>
                  ))}
              </div>

                <p className="text-sm text-center text-slate-600">
                    <RouterLink to="/" className="font-medium text-indigo-600 hover:text-indigo-500">
                        返回首页
                    </RouterLink>
                </p>
                <p className="text-sm text-center text-slate-600">
                    还没有账户？{' '}
                    <RouterLink to="/register" className="font-medium text-indigo-600 hover:text-indigo-500">
                        立即注册
                    </RouterLink>
                </p>
            </div>
        </div>
    );
};

export default LoginPage;