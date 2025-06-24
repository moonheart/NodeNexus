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
    const [oauthError, setOauthError] = useState<string | null>(null);

    useEffect(() => {
        if (isAuthenticated) {
            navigate('/', { replace: true });
        }
    }, [isAuthenticated, navigate]);

    useEffect(() => {
        const params = new URLSearchParams(window.location.search);
        const errorParam = params.get('error');
        if (errorParam) {
            setOauthError(decodeURIComponent(errorParam));
            // Clean the URL
            window.history.replaceState({}, document.title, "/login");
        }

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

                    {oauthError && (
                        <div className="p-3 text-sm text-amber-700 bg-amber-100 rounded-md">
                            {oauthError}
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
                  {providers
                    .filter(provider => provider && provider.name)
                    .map((provider) => (
                      <button
                          key={provider.name}
                          type="button"
                          onClick={() => window.location.href = `/api/auth/${provider.name}/login`}
                          className="w-full flex justify-center items-center py-2 px-4 border border-slate-300 rounded-md shadow-sm text-sm font-medium text-slate-700 bg-white hover:bg-slate-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 mb-2"
                      >
                          {/* You can add specific icons based on provider_name */}
                            <img
                                src={provider.iconUrl} // Assuming you have icons in a public/icons folder
                                alt={`${provider.name} icon`}
                                className="w-5 h-5 mr-2"/>
                          使用 {provider.name.charAt(0).toUpperCase() + provider.name.slice(1)} 登录
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