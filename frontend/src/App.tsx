import React, { useEffect } from 'react'; // Added useEffect
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { ThemeProvider, createTheme, CssBaseline, AppBar, Toolbar, Typography, Button, Box } from '@mui/material';
import { Link as RouterLink } from 'react-router-dom';

import HomePage from './pages/HomePage';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import NotFoundPage from './pages/NotFoundPage';
import VpsDetailPage from './pages/VpsDetailPage'; // Import the new page
import ProtectedRoute from './components/ProtectedRoute';
import { useAuthStore } from './store/authStore';
import { useServerListStore } from './store/serverListStore'; // Added serverListStore

// 一个简单的亮色主题
const lightTheme = createTheme({
  palette: {
    mode: 'light',
  },
});

function App() {
  const { isAuthenticated, logout } = useAuthStore();
  const { initializeWebSocket, disconnectWebSocket } = useServerListStore();

  useEffect(() => {
    if (isAuthenticated) {
      console.log('App.tsx: User is authenticated, initializing WebSocket.');
      initializeWebSocket();
    } else {
      console.log('App.tsx: User is not authenticated, disconnecting WebSocket.');
      disconnectWebSocket();
    }

    // Cleanup on component unmount or when isAuthenticated changes to false
    return () => {
      // This cleanup will also run if isAuthenticated becomes false before unmount
      // console.log('App.tsx: Cleanup effect, disconnecting WebSocket.');
      // disconnectWebSocket(); // Covered by the else block, but good for unmount
    };
  }, [isAuthenticated, initializeWebSocket, disconnectWebSocket]);

  // Additional cleanup specifically for logout action
  const handleLogout = () => {
    logout(); // This will set isAuthenticated to false, triggering the useEffect above
    // disconnectWebSocket(); // Explicitly disconnect, though useEffect should also handle it.
                           // The useEffect is preferred as it centralizes the logic based on isAuthenticated.
  };

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
                <Button color="inherit" onClick={handleLogout}> {/* Changed to handleLogout */}
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
                <Route path="/vps/:vpsId" element={<VpsDetailPage />} /> {/* Add new route for VPS detail */}
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
