import React from 'react';
import { Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';

// interface ProtectedRouteProps { // No props needed for now
//     // children?: React.ReactNode; // Outlet handles children now
// }

const ProtectedRoute: React.FC = () => { // Removed ProtectedRouteProps
    const { isAuthenticated } = useAuthStore();

    if (!isAuthenticated) {
        // 用户未认证，重定向到登录页面
        return <Navigate to="/login" replace />;
    }

    return <Outlet />; // 如果已认证，渲染子路由
};

export default ProtectedRoute;