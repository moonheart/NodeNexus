import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { useServerListStore } from './store/serverListStore';
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
import ServerManagementPage from './pages/ServerManagementPage';
import ProtectedRoute from './components/ProtectedRoute';
import Layout from './components/Layout'; // Import the new Layout component
import SettingsLayout from './components/SettingsLayout'; // Import the SettingsLayout component
import { useAuthStore } from './store/authStore';

function App() {
  const { isAuthenticated } = useAuthStore();
  
  // Initialize the server list store to listen for auth changes.
  // This should only run once when the app component mounts.
  useEffect(() => {
    useServerListStore.getState().init();
  }, []);

  return (
    <Router>
      <Toaster position="top-center" reverseOrder={false} />
      <Routes>
        {/* Routes that should not have the main layout but use a different layout or none */}
        <Route path="/login" element={isAuthenticated ? <Navigate to="/" replace /> : <LoginPage />} />
        <Route path="/register" element={isAuthenticated ? <Navigate to="/" replace /> : <RegisterPage />} />

        {/* Routes within the main layout */}
        <Route element={<Layout />}>
          {/* Publicly accessible routes */}
          <Route path="/" element={<HomePage />} />
          <Route path="/vps/:vpsId" element={<VpsDetailPage />} />

          {/* Protected routes */}
          <Route element={<ProtectedRoute />}>
            <Route path="/tasks" element={<BatchCommandPage />} />
            <Route path="/monitors" element={<ServiceMonitoringPage />} />
            <Route path="/monitors/:monitorId" element={<ServiceMonitorDetailPage />} />
            <Route path="/servers" element={<ServerManagementPage />} />
            
            {/* Settings Section with Nested Routes */}
            <Route path="/settings" element={<SettingsLayout />}>
              <Route index element={<Navigate to="/settings/global" replace />} />
              <Route path="global" element={<SettingsPage />} />
              <Route path="notifications" element={<NotificationsPage />} />
              <Route path="alerts" element={<SettingsPage />} /> {/* Placeholder, assuming alerts are on settings page for now */}
              <Route path="tags"element={<TagManagementPage />} />
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
