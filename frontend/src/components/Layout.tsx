import React from 'react';
import { Outlet } from 'react-router-dom';
import Navbar from './Navbar';

const Layout: React.FC = () => {
  return (
    <div className="min-h-screen flex flex-col bg-slate-50">
      <Navbar />
      <main className="flex-grow container mx-auto px-4 py-6 sm:px-6 lg:px-8">
        <Outlet />
      </main>
      <footer className="bg-white text-slate-500 text-center p-4 text-sm mt-8 border-t">
        © {new Date().getFullYear()} VPS Monitor. All rights reserved.
      </footer>
    </div>
  );
};

export default Layout;