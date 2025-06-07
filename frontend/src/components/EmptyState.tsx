import React from 'react';
import { PlusCircle } from 'lucide-react';

interface EmptyStateProps {
  title: string;
  message: string;
  buttonText: string;
  onButtonClick: () => void;
}

const EmptyState: React.FC<EmptyStateProps> = ({ title, message, buttonText, onButtonClick }) => {
  return (
    <div className="text-center py-16 px-6 bg-slate-50 rounded-lg border-2 border-dashed border-slate-200">
      <div className="mx-auto w-16 h-16 text-slate-400 bg-slate-100 rounded-full flex items-center justify-center">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-8 h-8">
          <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
        </svg>
      </div>
      <h3 className="mt-4 text-lg font-semibold text-slate-800">{title}</h3>
      <p className="mt-2 text-sm text-slate-500">{message}</p>
      <div className="mt-6">
        <button
          type="button"
          onClick={onButtonClick}
          className="btn btn-primary"
        >
          <PlusCircle className="w-4 h-4 mr-2" />
          {buttonText}
        </button>
      </div>
    </div>
  );
};

export default EmptyState;