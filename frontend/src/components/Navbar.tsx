import React from 'react';
import { Link, NavLink } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import { ServerIcon } from './Icons';
import UserMenu from './UserMenu';
import { ThemeToggle } from './ThemeToggle';
import { Button } from '@/components/ui/button';

const Navbar: React.FC = () => {
  const { isAuthenticated } = useAuthStore();

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 max-w-screen-2xl items-center">
        <div className="mr-4 hidden md:flex">
          <Link to="/" className="mr-6 flex items-center space-x-2">
            <ServerIcon className="h-6 w-6" />
            <span className="hidden font-bold sm:inline-block">VPS Monitor</span>
          </Link>
          <nav className="flex items-center gap-4 text-sm lg:gap-6">
            {isAuthenticated && (
              <>
                <NavLink to="/servers">
                  {({ isActive }) => (
                    <Button variant={isActive ? "secondary" : "ghost"} asChild>
                      <Link to="/servers">服务器管理</Link>
                    </Button>
                  )}
                </NavLink>
                <NavLink to="/tasks">
                  {({ isActive }) => (
                    <Button variant={isActive ? "secondary" : "ghost"} asChild>
                      <Link to="/tasks">任务</Link>
                    </Button>
                  )}
                </NavLink>
                <NavLink to="/monitors">
                  {({ isActive }) => (
                    <Button variant={isActive ? "secondary" : "ghost"} asChild>
                      <Link to="/monitors">服务监控</Link>
                    </Button>
                  )}
                </NavLink>
                <NavLink to="/settings">
                  {({ isActive }) => (
                    <Button variant={isActive ? "secondary" : "ghost"} asChild>
                      <Link to="/settings">设置</Link>
                    </Button>
                  )}
                </NavLink>
              </>
            )}
          </nav>
        </div>
        <div className="flex flex-1 items-center justify-between space-x-2 md:justify-end">
          <nav className="flex items-center gap-2">
            {isAuthenticated ? (
              <UserMenu />
            ) : (
              <>
                <Button variant="ghost" asChild>
                  <Link to="/login">登录</Link>
                </Button>
                <Button asChild>
                  <Link to="/register">注册</Link>
                </Button>
              </>
            )}
            <ThemeToggle />
          </nav>
        </div>
      </div>
    </header>
  );
};

export default Navbar;