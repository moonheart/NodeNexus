import React from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { ThemeProvider, createTheme, CssBaseline, AppBar, Toolbar, Typography, Button, Box } from '@mui/material';
import { Link as RouterLink } from 'react-router-dom';

import HomePage from './pages/HomePage';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import NotFoundPage from './pages/NotFoundPage';
import ProtectedRoute from './components/ProtectedRoute';
import { useAuthStore } from './store/authStore';

// 一个简单的亮色主题
const lightTheme = createTheme({
  palette: {
    mode: 'light',
  },
});

function App() {
  const { isAuthenticated, logout } = useAuthStore();

  return (
    <ThemeProvider theme={lightTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex', flexDirection: 'column', minHeight: '100vh' }}>
          <AppBar position="static">
            <Toolbar>
              <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
                <RouterLink to="/" style={{ textDecoration: 'none', color: 'inherit' }}>
                  我的应用
                </RouterLink>
              </Typography>
              {isAuthenticated ? (
                <Button color="inherit" onClick={logout}>
                  登出
                </Button>
              ) : (
                <>
                  <Button color="inherit" component={RouterLink} to="/login">
                    登录
                  </Button>
                  <Button color="inherit" component={RouterLink} to="/register">
                    注册
                  </Button>
                </>
              )}
            </Toolbar>
          </AppBar>
          <Box
            component="main"
            sx={{
              flexGrow: 1,
              p: 3,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center', // Center content horizontally
              justifyContent: 'center' // Center content vertically
            }}
          >
            {/* The pages (LoginPage, RegisterPage, etc.) should handle their own internal centering if needed */}
            <Routes>
              <Route path="/login" element={isAuthenticated ? <Navigate to="/" replace /> : <LoginPage />} />
              <Route path="/register" element={isAuthenticated ? <Navigate to="/" replace /> : <RegisterPage />} />
              
              {/* 受保护的路由 */}
              <Route element={<ProtectedRoute />}>
                <Route path="/" element={<HomePage />} />
                {/* 在这里添加其他受保护的路由 */}
              </Route>
              
              <Route path="*" element={<NotFoundPage />} />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
