import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';

import HomePage from './pages/HomePage';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import NotFoundPage from './pages/NotFoundPage';
import VpsDetailPage from './pages/VpsDetailPage';
import SettingsPage from './pages/SettingsPage';
import TagManagementPage from './pages/TagManagementPage';
import BatchCommandPage from './pages/BatchCommandPage'; // Import the new page
import NotificationsPage from './pages/NotificationsPage'; // Import the new page
import ServiceMonitoringPage from './pages/ServiceMonitoringPage'; // Import the new page
import ServiceMonitorDetailPage from './pages/ServiceMonitorDetailPage'; // Import the new page
import ProtectedRoute from './components/ProtectedRoute';
import Layout from './components/Layout'; // Import the new Layout component
import SettingsLayout from './components/SettingsLayout'; // Import the SettingsLayout component
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
      <Toaster position="top-center" reverseOrder={false} />
      <Routes>
        {/* Routes that should not have the main layout */}
        <Route path="/login" element={isAuthenticated ? <Navigate to="/" replace /> : <LoginPage />} />
        <Route path="/register" element={isAuthenticated ? <Navigate to="/" replace /> : <RegisterPage />} />

        {/* Routes protected and within the main layout */}
        <Route element={<ProtectedRoute />}>
          <Route element={<Layout />}>
            <Route path="/" element={<HomePage />} />
            <Route path="/vps/:vpsId" element={<VpsDetailPage />} />
            <Route path="/tasks" element={<BatchCommandPage />} />
            <Route path="/monitors" element={<ServiceMonitoringPage />} />
            <Route path="/monitors/:monitorId" element={<ServiceMonitorDetailPage />} />
            
            {/* Settings Section with Nested Routes */}
            <Route path="/settings" element={<SettingsLayout />}>
              <Route index element={<Navigate to="/settings/global" replace />} />
              <Route path="global" element={<SettingsPage />} />
              <Route path="notifications" element={<NotificationsPage />} />
              <Route path="alerts" element={<SettingsPage />} /> {/* Placeholder, assuming alerts are on settings page for now */}
              <Route path="tags" element={<TagManagementPage />} />
              <Route path="scripts" element={<div>Script Management Page (TODO)</div>} />
              <Route path="account" element={<div>Account Settings Page (TODO)</div>} />
            </Route>
          </Route>
        </Route>

        {/* Catch-all 404 route */}
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Router>
  );
}

export default App;
