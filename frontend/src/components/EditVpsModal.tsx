import React, { useState, useEffect, useMemo } from 'react';
import Select from 'react-select';
import CreatableSelect from 'react-select/creatable';
import { updateVps } from '../services/vpsService';
import type { VpsListItemResponse } from '../types';
import axios from 'axios';
import { useServerListStore } from '../store/serverListStore';
import { X } from 'lucide-react';

interface EditVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vps: VpsListItemResponse | null;
  allVps: VpsListItemResponse[]; // Keep for group options for now
  onVpsUpdated: () => void; // Callback to trigger data refresh
}

const EditVpsModal: React.FC<EditVpsModalProps> = ({ isOpen, onClose, vps, allVps, onVpsUpdated }) => {
  const [name, setName] = useState('');
  const [group, setGroup] = useState<{ value: string; label: string } | null>(null);
  const [selectedTags, setSelectedTags] = useState<{ value: number; label: string }[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const allTags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const groupOptions = useMemo(() => {
    const allGroups = new Set(allVps.map(v => v.group).filter((g): g is string => !!g));
    return [...allGroups].map(g => ({ value: g, label: g }));
  }, [allVps]);

  const tagOptions = useMemo(() => {
    return allTags.map(tag => ({ value: tag.id, label: tag.name }));
  }, [allTags]);

  useEffect(() => {
    if (isOpen && fetchAllTags) {
      fetchAllTags();
    }
  }, [isOpen, fetchAllTags]);

  useEffect(() => {
    // When the modal is opened or the vps prop changes, initialize the form state.
    // This prevents live data from websockets (via the `servers` store) from
    // overwriting what the user is actively editing.
    if (isOpen && vps) {
      setName(vps.name || '');
      setGroup(vps.group ? { value: vps.group, label: vps.group } : null);
      setSelectedTags(vps.tags ? vps.tags.map(t => ({ value: t.id, label: t.name })) : []);
      setError(null);
      setIsLoading(false);
    }
  }, [vps, isOpen]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!vps) return;

    setIsLoading(true);
    setError(null);

    const payload = {
      name: name.trim(),
      group: group?.value, // Use undefined when group is null, not null
      tag_ids: selectedTags.map(t => t.value),
    };

    try {
      await updateVps(vps.id, payload);
      onVpsUpdated(); // Trigger refresh in parent component
      onClose(); // Close modal on success
    } catch (err: unknown) {
      console.error('Failed to update VPS:', err);
      let errorMessage = '更新VPS失败，请稍后再试。';
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        errorMessage = err.response.data.error;
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  if (!isOpen || !vps) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-slate-900/50 flex items-center justify-center z-50 transition-opacity duration-300">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-md m-4 transform transition-all duration-300">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold text-slate-800">编辑服务器信息</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 transition-colors">
            <X className="w-6 h-6" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <div>
              <label htmlFor="vpsName" className="block text-sm font-medium text-slate-700 mb-1">名称</label>
              <input
                type="text"
                id="vpsName"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
                required
              />
            </div>
            <div>
              <label htmlFor="vpsGroup" className="block text-sm font-medium text-slate-700 mb-1">分组</label>
              <CreatableSelect
                isClearable
                options={groupOptions}
                value={group}
                onChange={(newValue) => setGroup(newValue)}
                placeholder="选择或创建一个分组..."
              />
            </div>
            <div>
              <label htmlFor="vpsTags" className="block text-sm font-medium text-slate-700 mb-1">标签</label>
              <Select
                isMulti
                options={tagOptions}
                value={selectedTags}
                onChange={(newValue) => setSelectedTags(Array.from(newValue))}
                placeholder="选择标签..."
                closeMenuOnSelect={false}
              />
            </div>
          </div>

          {error && <p className="text-red-500 text-sm mt-4">错误: {error}</p>}

          <div className="mt-6 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="bg-slate-200 hover:bg-slate-300 text-slate-800 font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={isLoading}
              className="bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150 disabled:bg-indigo-400 disabled:cursor-not-allowed"
            >
              {isLoading ? '保存中...' : '保存更改'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default EditVpsModal;
