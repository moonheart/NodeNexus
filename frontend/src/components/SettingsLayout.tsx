import React from 'react';
import { NavLink, Outlet, useLocation } from 'react-router-dom';
import { SlidersHorizontal, Bell, AlertTriangle, Tags, ScrollText, User, KeyRound, Palette } from 'lucide-react';
import { Button } from '@/components/ui/button';

const settingsGroups = [
  {
    title: '系统管理',
    links: [
      { name: '全局配置', to: '/settings/global', icon: SlidersHorizontal },
      { name: '通知渠道', to: '/settings/notifications', icon: Bell },
      { name: 'OAuth 配置', to: '/settings/oauth', icon: KeyRound },
    ],
  },
  {
    title: '功能配置',
    links: [
      { name: '告警规则', to: '/settings/alerts', icon: AlertTriangle },
      { name: '标签管理', to: '/settings/tags', icon: Tags },
      { name: '脚本管理', to: '/settings/scripts', icon: ScrollText },
    ],
  },
  {
    title: '个人设置',
    links: [
      { name: '账户信息', to: '/settings/account', icon: User },
      { name: '外观设置', to: '/settings/appearance', icon: Palette },
    ],
  },
];

const SettingsLayout: React.FC = () => {
  const location = useLocation();

  return (
    <div className="grid md:grid-cols-[220px_1fr] lg:grid-cols-[280px_1fr] gap-8 items-start">
      <aside className="hidden md:flex flex-col gap-4">
        <nav className="grid gap-2 text-sm text-muted-foreground">
          {settingsGroups.map((group) => (
            <div key={group.title}>
              <h3 className="px-4 text-xs font-semibold uppercase text-muted-foreground/80 tracking-wider mb-2">
                {group.title}
              </h3>
              <div className="grid gap-1">
                {group.links.map((link) => (
                  <Button
                    key={link.name}
                    variant={location.pathname === link.to ? 'secondary' : 'ghost'}
                    className="w-full justify-start"
                    asChild
                  >
                    <NavLink to={link.to}>
                      <link.icon className="mr-2 h-4 w-4" />
                      {link.name}
                    </NavLink>
                  </Button>
                ))}
              </div>
            </div>
          ))}
        </nav>
      </aside>
      <main>
        <Outlet />
      </main>
    </div>
  );
};

export default SettingsLayout;