import React, { useState, useEffect } from 'react';
import { useNavigate, Link as RouterLink } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import { getAuthProviders } from '../services/authService';
import type { LoginRequest, AuthProvider } from '../services/authService';
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Separator } from "@/components/ui/separator";
import { Loader2, AlertCircle } from "lucide-react";

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
        <div className="flex items-center justify-center min-h-screen bg-background">
            <Card className="w-full max-w-md">
                <CardHeader className="text-center">
                    <CardTitle className="text-2xl">登录您的账户</CardTitle>
                    <CardDescription>输入您的凭据以访问您的仪表板。</CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="username">用户名</Label>
                            <Input
                                id="username"
                                type="text"
                                placeholder="例如: admin"
                                required
                                value={username}
                                onChange={(e) => setUsername(e.target.value)}
                                autoComplete="username"
                            />
                        </div>
                        <div className="space-y-2">
                            <Label htmlFor="password">密码</Label>
                            <Input
                                id="password"
                                type="password"
                                required
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                autoComplete="current-password"
                            />
                        </div>

                        {error && (
                            <Alert variant="destructive">
                                <AlertCircle className="h-4 w-4" />
                                <AlertDescription>{error}</AlertDescription>
                            </Alert>
                        )}

                        {oauthError && (
                            <Alert variant="default">
                                <AlertCircle className="h-4 w-4" />
                                <AlertDescription>{oauthError}</AlertDescription>
                            </Alert>
                        )}

                        <Button type="submit" className="w-full" disabled={isLoading}>
                            {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                            {isLoading ? '登录中...' : '登录'}
                        </Button>
                    </form>

                    {providers.length > 0 && (
                        <>
                            <div className="relative my-6">
                                <Separator />
                                <div className="absolute inset-0 flex items-center">
                                    <span className="bg-background px-2 text-xs text-muted-foreground">
                                        或者通过以下方式继续
                                    </span>
                                </div>
                            </div>
                            <div className="space-y-2">
                                {providers
                                    .filter(provider => provider && provider.name)
                                    .map((provider) => (
                                        <Button
                                            key={provider.name}
                                            variant="outline"
                                            className="w-full"
                                            onClick={() => window.location.href = `/api/auth/${provider.name}/login`}
                                        >
                                            <img
                                                src={provider.iconUrl}
                                                alt={`${provider.name} icon`}
                                                className="w-5 h-5 mr-2"
                                            />
                                            使用 {provider.name.charAt(0).toUpperCase() + provider.name.slice(1)} 登录
                                        </Button>
                                    ))}
                            </div>
                        </>
                    )}
                </CardContent>
                <CardFooter className="flex flex-col items-center space-y-2">
                     <p className="text-sm text-muted-foreground">
                        <RouterLink to="/" className="underline underline-offset-4 hover:text-primary">
                            返回首页
                        </RouterLink>
                    </p>
                    <p className="text-sm text-muted-foreground">
                        还没有账户？{' '}
                        <RouterLink to="/register" className="underline underline-offset-4 hover:text-primary">
                            立即注册
                        </RouterLink>
                    </p>
                </CardFooter>
            </Card>
        </div>
    );
};

export default LoginPage;