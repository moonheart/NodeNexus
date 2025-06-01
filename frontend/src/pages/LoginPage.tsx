import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, TextField, Container, Typography, Box, Alert } from '@mui/material';
import { useAuthStore } from '../store/authStore';
import type { LoginRequest } from '../services/authService'; // 使用 service 中定义的类型
import { Link as RouterLink } from 'react-router-dom'; // 引入 RouterLink

const LoginPage: React.FC = () => {
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const { login, isLoading, error, isAuthenticated, clearAuthError } = useAuthStore();
    const navigate = useNavigate();

    React.useEffect(() => {
        if (isAuthenticated) {
            navigate('/', { replace: true });
        }
    }, [isAuthenticated, navigate]);

    const handleSubmit = async (event: React.FormEvent) => {
        event.preventDefault();
        clearAuthError(); // 清除之前的错误

        const credentials: LoginRequest = { email, password };

        try {
            await login(credentials);
            // 登录成功后，isAuthenticated 状态会更新，useEffect 会处理跳转
            // navigate('/'); // 不再需要在这里显式跳转
        } catch (err) {
            // 错误已在 authStore 中处理并设置
            console.error('Login failed on page:', err);
        }
    };

    return (
        <Container component="main" maxWidth="xs">
            <Box
                sx={{
                    marginTop: 8,
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                }}
            >
                <Typography component="h1" variant="h5">
                    登录
                </Typography>
                <Box component="form" onSubmit={handleSubmit} noValidate sx={{ mt: 1 }}>
                    <TextField
                        margin="normal"
                        required
                        fullWidth
                        id="email"
                        label="邮箱地址"
                        name="email"
                        autoComplete="email"
                        autoFocus
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        error={!!error} // 简易错误判断
                    />
                    <TextField
                        margin="normal"
                        required
                        fullWidth
                        name="password"
                        label="密码"
                        type="password"
                        id="password"
                        autoComplete="current-password"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        error={!!error} // 简易错误判断
                    />
                    {error && (
                        <Alert severity="error" sx={{ mt: 2, width: '100%' }}>
                            {error}
                        </Alert>
                    )}
                    <Button
                        type="submit"
                        fullWidth
                        variant="contained"
                        sx={{ mt: 3, mb: 2 }}
                        disabled={isLoading}
                    >
                        {isLoading ? '登录中...' : '登录'}
                    </Button>
                    <Box sx={{ mt: 2, textAlign: 'center' }}>
                        <Typography variant="body2">
                            还没有账户？{' '}
                            <RouterLink to="/register" style={{ textDecoration: 'none' }}>
                                立即注册
                            </RouterLink>
                        </Typography>
                    </Box>
                </Box>
            </Box>
        </Container>
    );
};

export default LoginPage;