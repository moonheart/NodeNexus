import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';

import HomePage from './pages/HomePage';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import NotFoundPage from './pages/NotFoundPage';
import VpsDetailPage from './pages/VpsDetailPage';
import ProtectedRoute from './components/ProtectedRoute';
import Layout from './components/Layout'; // Import the new Layout component
import { useAuthStore } from './store/authStore';
import { useServerListStore } from './store/serverListStore';

function App() {
  const { isAuthenticated } = useAuthStore();
  const { initializeWebSocket, disconnectWebSocket } = useServerListStore();

  useEffect(() => {
    if (isAuthenticated) {
      console.log('App.tsx: User is authenticated, initializing WebSocket.');
      initializeWebSocket();
    } else {
      console.log('App.tsx: User is not authenticated, disconnecting WebSocket.');
      disconnectWebSocket();
    }
    // The effect will re-run if isAuthenticated changes, handling both login and logout.
  }, [isAuthenticated, initializeWebSocket, disconnectWebSocket]);

  return (
    <Router>
      <Routes>
        {/* Routes that should not have the main layout */}
        <Route path="/login" element={isAuthenticated ? <Navigate to="/" replace /> : <LoginPage />} />
        <Route path="/register" element={isAuthenticated ? <Navigate to="/" replace /> : <RegisterPage />} />

        {/* Routes protected and within the main layout */}
        <Route element={<ProtectedRoute />}>
          <Route element={<Layout />}>
            <Route path="/" element={<HomePage />} />
            <Route path="/vps/:vpsId" element={<VpsDetailPage />} />
            {/* Add other protected routes that need the layout here */}
          </Route>
        </Route>

        {/* Catch-all 404 route */}
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Router>
  );
}

export default App;
