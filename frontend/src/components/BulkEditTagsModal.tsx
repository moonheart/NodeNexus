import React, { useState, useEffect, useMemo } from 'react';
import Select from 'react-select';
import { useServerListStore } from '../store/serverListStore';
import * as tagService from '../services/tagService';

interface BulkEditTagsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vpsIds: number[];
  onTagsUpdated: () => void; // Callback to trigger potential refreshes
}

const BulkEditTagsModal: React.FC<BulkEditTagsModalProps> = ({ isOpen, onClose, vpsIds, onTagsUpdated }) => {
  const [tagsToAdd, setTagsToAdd] = useState<{ value: number; label: string }[]>([]);
  const [tagsToRemove, setTagsToRemove] = useState<{ value: number; label: string }[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const allTags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  useEffect(() => {
    if (isOpen) {
      // Fetch the latest tags when the modal opens
      fetchAllTags();
      // Reset state
      setTagsToAdd([]);
      setTagsToRemove([]);
      setError(null);
      setIsLoading(false);
    }
  }, [isOpen, fetchAllTags]);

  const tagOptions = useMemo(() => {
    return allTags.map(tag => ({ value: tag.id, label: tag.name, color: tag.color }));
  }, [allTags]);

  // Ensure a tag cannot be in both "add" and "remove" lists
  const addOptions = useMemo(() => tagOptions.filter(opt => !tagsToRemove.some(r => r.value === opt.value)), [tagOptions, tagsToRemove]);
  const removeOptions = useMemo(() => tagOptions.filter(opt => !tagsToAdd.some(a => a.value === opt.value)), [tagOptions, tagsToAdd]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);

    try {
      await tagService.bulkUpdateVpsTags({
        vpsIds: vpsIds,
        addTagIds: tagsToAdd.map(t => t.value),
        removeTagIds: tagsToRemove.map(t => t.value),
      });
      onTagsUpdated();
      onClose();
    } catch (err) {
      console.error('Failed to bulk update tags:', err);
      setError('An error occurred. Please try again.');
    } finally {
      setIsLoading(false);
    }
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-slate-900/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-md m-4">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold text-slate-800">Bulk Edit Tags for {vpsIds.length} Servers</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600">&times;</button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <div>
              <label htmlFor="tagsToAdd" className="block text-sm font-medium text-slate-700 mb-1">Tags to Add</label>
              <Select
                isMulti
                options={addOptions}
                value={tagsToAdd}
                onChange={(newValue) => setTagsToAdd(Array.from(newValue))}
                placeholder="Select tags to add..."
                closeMenuOnSelect={false}
              />
            </div>
            <div>
              <label htmlFor="tagsToRemove" className="block text-sm font-medium text-slate-700 mb-1">Tags to Remove</label>
              <Select
                isMulti
                options={removeOptions}
                value={tagsToRemove}
                onChange={(newValue) => setTagsToRemove(Array.from(newValue))}
                placeholder="Select tags to remove..."
                closeMenuOnSelect={false}
              />
            </div>
          </div>

          {error && <p className="text-red-500 text-sm mt-4">Error: {error}</p>}

          <div className="mt-6 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="bg-slate-200 hover:bg-slate-300 text-slate-800 font-semibold py-2 px-4 rounded-lg shadow-sm"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isLoading}
              className="bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-sm disabled:bg-indigo-400"
            >
              {isLoading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default BulkEditTagsModal;