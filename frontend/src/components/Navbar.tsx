import React from 'react';
import { Link, NavLink, useNavigate } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import { ServerIcon } from './Icons';

const Navbar: React.FC = () => {
  const { isAuthenticated, logout } = useAuthStore();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    navigate('/login'); // Redirect to login page after logout
  };

  return (
    <header className="fixed top-0 left-0 right-0 z-50 bg-white/80 backdrop-blur-lg border-b border-slate-200/80 shadow-sm">
      <div className="container mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <div className="flex-shrink-0">
            <Link to="/" className="flex items-center space-x-2">
              <ServerIcon className="h-8 w-8 text-indigo-600" />
              <span className="text-xl font-semibold text-slate-800">VPS Monitor</span>
            </Link>
          </div>
          <nav className="flex items-center space-x-4">
            {isAuthenticated ? (
              <>
                <NavLink
                  to="/servers"
                  className={({ isActive }) =>
                    `px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      isActive ? 'bg-slate-100 text-indigo-600' : 'text-slate-700 hover:bg-slate-100'
                    }`
                  }
                >
                  服务器管理
                </NavLink>
                <NavLink
                  to="/tasks"
                  className={({ isActive }) =>
                    `px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      isActive ? 'bg-slate-100 text-indigo-600' : 'text-slate-700 hover:bg-slate-100'
                    }`
                  }
                >
                  任务
                </NavLink>
                <NavLink
                  to="/monitors"
                  className={({ isActive }) =>
                    `px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      isActive ? 'bg-slate-100 text-indigo-600' : 'text-slate-700 hover:bg-slate-100'
                    }`
                  }
                >
                  服务监控
                </NavLink>
                <NavLink
                  to="/settings"
                  className={({ isActive }) =>
                    `px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                      isActive ? 'bg-slate-100 text-indigo-600' : 'text-slate-700 hover:bg-slate-100'
                    }`
                  }
                >
                  设置
                </NavLink>
                <button
                  onClick={handleLogout}
                  className="px-3 py-2 rounded-md text-sm font-medium text-slate-700 hover:bg-slate-100 transition-colors"
                >
                  登出
                </button>
              </>
            ) : (
              <>
                <Link
                  to="/login"
                  className="px-3 py-2 rounded-md text-sm font-medium text-slate-700 hover:bg-slate-100 transition-colors"
                >
                  登录
                </Link>
                <Link
                  to="/register"
                  className="px-3 py-2 rounded-md text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 transition-colors"
                >
                  注册
                </Link>
              </>
            )}
          </nav>
        </div>
      </div>
    </header>
  );
};

export default Navbar;