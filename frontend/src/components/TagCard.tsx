import React from 'react';
import type { Tag as TagType } from '../types';
import { Pencil, Trash2, Link as LinkIcon, Eye, EyeOff } from 'lucide-react';
import * as Icons from './Icons';

interface TagCardProps {
  tag: TagType;
  onEdit: (tag: TagType) => void;
  onDelete: (tagId: number) => void;
  onToggleVisibility: (tagId: number, isVisible: boolean) => void;
}

const TagCard: React.FC<TagCardProps> = ({ tag, onEdit, onDelete, onToggleVisibility }) => {
  const TagIcon = Icons[tag.icon as keyof typeof Icons] || null;

  return (
    <div className="bg-white rounded-lg shadow-md border border-slate-200 hover:shadow-lg transition-shadow duration-300 flex flex-col">
      {/* Header with color and name */}
      <div
        className="flex items-center justify-between p-3 rounded-t-lg"
        style={{ backgroundColor: tag.color, color: 'white' }} // Simple contrast, might need a better function
      >
        <div className="flex items-center gap-2">
          {TagIcon && <TagIcon className="w-5 h-5" />}
          <span className="font-bold text-lg">{tag.name}</span>
        </div>
        <div className="text-sm font-mono bg-black/20 px-2 py-1 rounded">
          ID: {tag.id}
        </div>
      </div>

      {/* Body with details */}
      <div className="p-4 space-y-3 flex-grow">
        <div className="flex items-center justify-between text-sm text-slate-600">
          <span>Usage Count:</span>
          <span className="font-semibold text-slate-800">{tag.vpsCount}</span>
        </div>
        {tag.url && (
          <div className="flex items-center justify-between text-sm text-slate-600">
            <span>Associated URL:</span>
            <a href={tag.url} target="_blank" rel="noopener noreferrer" className="flex items-center gap-1 text-indigo-600 hover:underline">
              <LinkIcon className="w-4 h-4" />
              <span>Link</span>
            </a>
          </div>
        )}
      </div>

      {/* Footer with actions */}
      <div className="p-3 bg-slate-50 rounded-b-lg flex items-center justify-between border-t">
        <div className="flex items-center gap-2">
            <button 
                onClick={() => onToggleVisibility(tag.id, !tag.isVisible)}
                className={`p-2 rounded-full transition-colors ${tag.isVisible ? 'text-slate-700 hover:bg-slate-200' : 'text-slate-400 hover:bg-slate-200'}`}
                title={tag.isVisible ? 'Visible' : 'Hidden'}
            >
                {tag.isVisible ? <Eye className="w-5 h-5" /> : <EyeOff className="w-5 h-5" />}
            </button>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => onEdit(tag)} className="btn btn-ghost btn-sm p-2">
            <Pencil className="w-4 h-4" />
          </button>
          <button onClick={() => onDelete(tag.id)} className="btn btn-ghost btn-sm p-2 text-red-500 hover:text-red-700">
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
};

export default TagCard;