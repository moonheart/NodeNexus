import React from 'react';
import { Link, NavLink } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';
import { ServerIcon } from './Icons';
import UserMenu from './UserMenu';
import { ThemeToggle } from './ThemeToggle';
import { Button, buttonVariants } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const navLinks = [
  { to: '/servers', label: '服务器管理' },
  { to: '/tasks', label: '任务' },
  { to: '/monitors', label: '服务监控' },
  { to: '/settings', label: '设置' },
];

const Navbar: React.FC = () => {
  const { isAuthenticated } = useAuthStore();

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto flex h-16 max-w-screen-2xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center space-x-8">
          <div className="flex-shrink-0">
            <Link to="/" className="flex items-center space-x-2">
              <ServerIcon className="h-8 w-8 text-primary" />
              <span className="text-xl font-semibold text-foreground">NodeNexus</span>
            </Link>
          </div>
          <nav className="hidden items-center space-x-1 md:flex">
            {isAuthenticated && (
              <>
                {navLinks.map((link) => (
                  <NavLink
                    key={link.to}
                    to={link.to}
                    className={({ isActive }) =>
                      cn(
                        buttonVariants({ variant: isActive ? 'default' : 'ghost', size: 'sm' }),
                        'h-8',
                      )
                    }
                  >
                    {link.label}
                  </NavLink>
                ))}
              </>
            )}
          </nav>
        </div>
        <div className="flex items-center space-x-2">
          {isAuthenticated ? (
            <UserMenu />
          ) : (
            <>
              <Button asChild variant="ghost" size="sm">
                <Link to="/login">登录</Link>
              </Button>
              <Button asChild size="sm">
                <Link to="/register">注册</Link>
              </Button>
            </>
          )}
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
};

export default Navbar;