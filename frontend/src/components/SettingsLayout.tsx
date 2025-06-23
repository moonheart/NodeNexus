import React from 'react';
import { NavLink, Outlet } from 'react-router-dom';
import { SlidersHorizontal, Bell, AlertTriangle, Tags, ScrollText, User, KeyRound } from 'lucide-react';

const settingsGroups = [
  {
    title: '系统管理',
    links: [
      { name: '全局配置', to: '/settings/global', icon: SlidersHorizontal },
      { name: '通知渠道', to: '/settings/notifications', icon: Bell },
      { name: 'OAuth 配置', to: '/settings/oauth', icon: KeyRound },
      // { name: '用户管理', to: '/settings/users', icon: Users }, // Planned
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
      // { name: 'API 密钥', to: '/settings/api-keys' }, // Planned
    ],
  },
];

const SettingsLayout: React.FC = () => {
  const baseLinkClass = "flex items-center px-3 py-2 text-slate-700 rounded-md text-sm font-medium";
  const activeLinkClass = "bg-slate-200";
  const inactiveLinkClass = "hover:bg-slate-100";

  return (
    <div className="container mx-auto p-4 flex space-x-8">
      <aside className="w-1/4 lg:w-1/5 xl:w-1/6 flex-shrink-0">
        <nav className="flex flex-col space-y-4">
          {settingsGroups.map((group) => (
            <div key={group.title}>
              <h3 className="px-3 text-xs font-semibold uppercase text-slate-500 tracking-wider mb-2">
                {group.title}
              </h3>
              <div className="space-y-1">
                {group.links.map((link) => (
                  <NavLink
                    key={link.name}
                    to={link.to}
                    className={({ isActive }) =>
                      `${baseLinkClass} ${isActive ? activeLinkClass : inactiveLinkClass}`
                    }
                  >
                    <link.icon className="mr-3 h-5 w-5 text-slate-500" />
                    <span>{link.name}</span>
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>
      </aside>
      <main className="flex-1 bg-white p-6 rounded-lg shadow-md">
        <Outlet />
      </main>
    </div>
  );
};

export default SettingsLayout;