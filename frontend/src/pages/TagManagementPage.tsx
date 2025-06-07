import React, { useState, useEffect } from 'react';
import { PlusCircle, Tag as TagIcon, Pencil, Trash2 } from 'lucide-react';
import * as tagService from '../services/tagService';
import type { Tag } from '../types';
import TagEditModal from '../components/TagEditModal';
import { useServerListStore } from '../store/serverListStore';

const TagManagementPage: React.FC = () => {
  const tags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);
  const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
  const [editingTag, setEditingTag] = useState<Tag | null>(null);

  useEffect(() => {
    const loadInitialTags = async () => {
      try {
        setIsLoading(true);
        await fetchAllTags();
        setError(null);
      } catch (err) {
        setError(err as Error);
      } finally {
        setIsLoading(false);
      }
    };
    loadInitialTags();
  }, [fetchAllTags]);

  const handleCreateClick = () => {
    setEditingTag(null);
    setIsModalOpen(true);
  };

  const handleEditClick = (tag: Tag) => {
    setEditingTag(tag);
    setIsModalOpen(true);
  };

  const handleDeleteClick = async (tagId: number) => {
    if (window.confirm('Are you sure you want to delete this tag? This action cannot be undone.')) {
      try {
        await tagService.deleteTag(tagId);
        await fetchAllTags(); // Refetch to update the list
      } catch (err) {
        setError(err as Error);
        console.error('Failed to delete tag:', err);
      }
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingTag(null);
  };

  const handleTagSaved = () => {
    fetchAllTags(); // Refetch tags after saving
  };

  if (isLoading) {
    return <div>Loading tags...</div>;
  }

  return (
    <div className="container mx-auto p-4">
      {error && <div className="alert alert-error shadow-lg mb-4"><div><span>Error: {error.message}</span></div></div>}
      
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-2xl font-bold">Tag Management</h1>
        <button className="btn btn-primary" onClick={handleCreateClick}>
          <PlusCircle className="w-4 h-4 mr-2" />
          Create Tag
        </button>
      </div>

      <div className="overflow-x-auto">
        <table className="table w-full">
          <thead>
            <tr>
              <th>Name</th>
              <th>Color</th>
              <th>Icon</th>
              <th>URL</th>
              <th>Visible</th>
              <th>Usage Count</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {tags?.map((tag: Tag) => (
              <tr key={tag.id}>
                <td><span className="font-bold">{tag.name}</span></td>
                <td>
                  <div className="flex items-center gap-2">
                    <div className="w-4 h-4 rounded-full border" style={{ backgroundColor: tag.color }} />
                    <span>{tag.color}</span>
                  </div>
                </td>
                <td>{tag.icon ? <TagIcon className="w-5 h-5" /> : 'None'}</td>
                <td>{tag.url ? <a href={tag.url} target="_blank" rel="noopener noreferrer" className="link link-primary">Link</a> : 'None'}</td>
                <td>{tag.isVisible ? 'Yes' : 'No'}</td>
                <td>{tag.vpsCount}</td>
                <td>
                  <div className="flex gap-2">
                    <button className="btn btn-ghost btn-sm" onClick={() => handleEditClick(tag)}><Pencil className="w-4 h-4" /></button>
                    <button className="btn btn-ghost btn-sm text-red-500" onClick={() => handleDeleteClick(tag.id)}><Trash2 className="w-4 h-4" /></button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <TagEditModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onTagSaved={handleTagSaved}
        tag={editingTag}
      />
    </div>
  );
};

export default TagManagementPage;