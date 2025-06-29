import React from 'react';
import { FileText } from 'lucide-react';

interface EmptyStateProps {
  title: string;
  message: string;
  action?: React.ReactNode;
}

const EmptyState: React.FC<EmptyStateProps> = ({ title, message, action }) => {
  return (
    <div className="text-center py-16 px-6 bg-slate-50/50 dark:bg-slate-900/20 rounded-lg border-2 border-dashed border-slate-200 dark:border-slate-800">
      <div className="mx-auto w-16 h-16 text-slate-400 dark:text-slate-500 bg-slate-100 dark:bg-slate-800/50 rounded-full flex items-center justify-center">
        <FileText className="w-8 h-8" />
      </div>
      <h3 className="mt-4 text-lg font-semibold text-slate-800 dark:text-slate-200">{title}</h3>
      <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">{message}</p>
      {action && (
        <div className="mt-6">
          {action}
        </div>
      )}
    </div>
  );
};

export default EmptyState;