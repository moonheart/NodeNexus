import React from 'react';

const TagSkeletonCard: React.FC = () => {
  return (
    <div className="bg-white rounded-lg shadow-md border border-slate-200 flex flex-col animate-pulse">
      {/* Header */}
      <div className="p-3 rounded-t-lg bg-slate-300">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-5 h-5 bg-slate-400 rounded"></div>
            <div className="w-24 h-6 bg-slate-400 rounded"></div>
          </div>
          <div className="w-12 h-6 bg-slate-400 rounded"></div>
        </div>
      </div>

      {/* Body */}
      <div className="p-4 space-y-3 flex-grow">
        <div className="flex items-center justify-between">
          <div className="w-20 h-4 bg-slate-300 rounded"></div>
          <div className="w-8 h-4 bg-slate-300 rounded"></div>
        </div>
        <div className="flex items-center justify-between">
          <div className="w-24 h-4 bg-slate-300 rounded"></div>
          <div className="w-12 h-4 bg-slate-300 rounded"></div>
        </div>
      </div>

      {/* Footer */}
      <div className="p-3 bg-slate-50 rounded-b-lg flex items-center justify-between border-t">
        <div className="w-8 h-8 bg-slate-200 rounded-full"></div>
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-slate-200 rounded"></div>
          <div className="w-8 h-8 bg-slate-200 rounded"></div>
        </div>
      </div>
    </div>
  );
};

export default TagSkeletonCard;