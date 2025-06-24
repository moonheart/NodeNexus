import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { useServerListStore } from './store/serverListStore';
import websocketService from './services/websocketService';
import HomePage from './pages/HomePage';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import NotFoundPage from './pages/NotFoundPage';
import VpsDetailPage from './pages/VpsDetailPage';
import GlobalSettingsPage from './pages/GlobalSettingsPage';
import AlertsSettingsPage from './pages/AlertsSettingsPage';
import TagManagementPage from './pages/TagManagementPage';
import BatchCommandPage from './pages/BatchCommandPage'; // Import the new page
import NotificationsPage from './pages/NotificationsPage'; // Import the new page
import ServiceMonitoringPage from './pages/ServiceMonitoringPage'; // Import the new page
import ServiceMonitorDetailPage from './pages/ServiceMonitorDetailPage'; // Import the new page
import ServerManagementPage from './pages/ServerManagementPage';
import AdminOAuthProvidersPage from './pages/AdminOAuthProvidersPage';
import ProtectedRoute from './components/ProtectedRoute';
import Layout from './components/Layout'; // Import the new Layout component
import SettingsLayout from './components/SettingsLayout';
import AccountSettingsPage from './pages/AccountSettingsPage'; // Import the new page
import AuthCallbackPage from './pages/AuthCallbackPage'; // Import the new callback page
import { useAuthStore } from './store/authStore';

function App() {
  const { isAuthenticated, token } = useAuthStore();
  
  // Initialize the server list store and establish WebSocket connection on app load.
  // This should only run once when the app component mounts.
  useEffect(() => {
    // Initialize stores that need to react to events or auth state
    useServerListStore.getState().init();

    // Establish the initial WebSocket connection based on the current auth state
    websocketService.connect(token);

    // The cleanup function will run when the App component unmounts.
    return () => {
      websocketService.disconnect();
    };
  }, []); // The empty dependency array ensures this effect runs only once on mount.

  return (
    <Router>
      <Toaster position="top-center" reverseOrder={false} />
      <Routes>
        {/* Routes that should not have the main layout but use a different layout or none */}
        <Route path="/login" element={isAuthenticated ? <Navigate to="/" replace /> : <LoginPage />} />
        <Route path="/register" element={isAuthenticated ? <Navigate to="/" replace /> : <RegisterPage />} />
        <Route path="/auth/callback" element={<AuthCallbackPage />} />

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
            {/* Settings Section with Nested Routes */}
            <Route path="/settings" element={<SettingsLayout />}>
              <Route index element={<Navigate to="/settings/global" replace />} />
              <Route path="global" element={<GlobalSettingsPage />} />
              <Route path="alerts" element={<AlertsSettingsPage />} />
              <Route path="notifications" element={<NotificationsPage />} />
              <Route path="tags"element={<TagManagementPage />} />
              <Route path="scripts" element={<div>Script Management Page (TODO)</div>} />
              <Route path="oauth" element={<AdminOAuthProvidersPage />} />
              <Route path="account" element={<AccountSettingsPage />} />
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
