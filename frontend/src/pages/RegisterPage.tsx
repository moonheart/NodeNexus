import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, TextField, Container, Typography, Box, Alert } from '@mui/material';
import { useAuthStore } from '../store/authStore';
import type { RegisterRequest } from '../services/authService'; // 使用 service 中定义的类型

const RegisterPage: React.FC = () => {
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const { register, isLoading, error, clearAuthError } = useAuthStore();
    const navigate = useNavigate();

    const handleSubmit = async (event: React.FormEvent) => {
        event.preventDefault();
        clearAuthError(); // 清除之前的错误

        const userData: RegisterRequest = { username, email, password };

        try {
            await register(userData);
            // 注册成功后，根据计划跳转到登录页
            navigate('/login');
        } catch (err) {
            // 错误已在 authStore 中处理并设置，这里不需要额外 setError
            // setError( (err as Error).message || '注册失败，请稍后再试。');
            console.error('Registration failed on page:', err);
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
                    注册
                </Typography>
                <Box component="form" onSubmit={handleSubmit} noValidate sx={{ mt: 1 }}>
                    <TextField
                        margin="normal"
                        required
                        fullWidth
                        id="username"
                        label="用户名"
                        name="username"
                        autoComplete="username"
                        autoFocus
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                        error={!!error && error.includes("用户名")} // 简易错误判断
                    />
                    <TextField
                        margin="normal"
                        required
                        fullWidth
                        id="email"
                        label="邮箱地址"
                        name="email"
                        autoComplete="email"
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        error={!!error && error.includes("邮箱")} // 简易错误判断
                    />
                    <TextField
                        margin="normal"
                        required
                        fullWidth
                        name="password"
                        label="密码"
                        type="password"
                        id="password"
                        autoComplete="new-password"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        error={!!error && error.includes("密码")} // 简易错误判断
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
                        {isLoading ? '注册中...' : '注册'}
                    </Button>
                </Box>
            </Box>
        </Container>
    );
};

export default RegisterPage;