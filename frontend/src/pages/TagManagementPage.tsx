import React, { useState, useEffect, useMemo } from 'react';
import { PlusCircle, Search } from 'lucide-react';
import toast from 'react-hot-toast';
import * as tagService from '../services/tagService';
import type { Tag, UpdateTagPayload } from '../types';
import { useServerListStore } from '../store/serverListStore';
import TagEditModal from '../components/TagEditModal';
import TagCard from '../components/TagCard';
import TagSkeletonCard from '../components/TagSkeletonCard';
import EmptyState from '../components/EmptyState';

const TagManagementPage: React.FC = () => {
  const tags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
  const [editingTag, setEditingTag] = useState<Tag | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    const loadInitialTags = async () => {
      try {
        setIsLoading(true);
        await fetchAllTags();
      } catch (err) {
        toast.error('Failed to load tags.');
        console.error(err);
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
    const tagToDelete = tags.find(t => t.id === tagId);
    const confirmMessage = tagToDelete && (tagToDelete.vpsCount ?? 0) > 0
      ? `This tag is used by ${tagToDelete.vpsCount} VPS(s). Deleting it will remove it from them. Are you sure?`
      : 'Are you sure you want to delete this tag?';

    if (window.confirm(confirmMessage)) {
      toast.promise(
        tagService.deleteTag(tagId).then(() => fetchAllTags()),
        {
          loading: 'Deleting tag...',
          success: 'Tag deleted successfully!',
          error: 'Failed to delete tag.',
        }
      );
    }
  };

  const handleToggleVisibility = async (tagId: number, isVisible: boolean) => {
    const originalTags = [...tags];
    const tagToUpdate = originalTags.find(t => t.id === tagId);
    if (!tagToUpdate) return;

    // Optimistic update
    const updatedTags = originalTags.map(t => t.id === tagId ? { ...t, isVisible } : t);
    useServerListStore.setState({ allTags: updatedTags });

    try {
      const payload: UpdateTagPayload = {
        name: tagToUpdate.name,
        color: tagToUpdate.color,
        icon: tagToUpdate.icon || undefined,
        url: tagToUpdate.url || undefined,
        is_visible: isVisible,
      };
      await tagService.updateTag(tagId, payload);
      toast.success(`Tag visibility updated.`);
      await fetchAllTags(); // re-sync with server
    } catch (error) {
      console.error("Failed to update visibility:", error);
      toast.error('Failed to update visibility.');
      useServerListStore.setState({ allTags: originalTags }); // Revert on error
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingTag(null);
  };

  const handleTagSaved = () => {
    fetchAllTags();
  };

  const filteredTags = useMemo(() => {
    if (!tags) return [];
    return tags.filter(tag =>
      tag.name.toLowerCase().includes(searchQuery.toLowerCase())
    );
  }, [tags, searchQuery]);

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => <TagSkeletonCard key={i} />)}
        </div>
      );
    }

    if (!tags || tags.length === 0) {
      return (
        <EmptyState
          title="No Tags Found"
          message="Get started by creating your first tag."
          buttonText="Create Tag"
          onButtonClick={handleCreateClick}
        />
      );
    }

    if (filteredTags.length === 0) {
        return <div className="text-center py-10">No tags match your search.</div>;
    }

    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {filteredTags.map((tag) => (
          <TagCard
            key={tag.id}
            tag={tag}
            onEdit={handleEditClick}
            onDelete={handleDeleteClick}
            onToggleVisibility={handleToggleVisibility}
          />
        ))}
      </div>
    );
  };

  return (
    <div className="container mx-auto p-4">
      <div className="flex flex-col sm:flex-row justify-between items-center mb-6 gap-4">
        <h1 className="text-3xl font-bold text-slate-800">Tag Management</h1>
        <div className="flex items-center gap-2 w-full sm:w-auto">
            <div className="relative w-full sm:w-64">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-400" />
                <input
                    type="text"
                    placeholder="Search tags..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="pl-10 pr-4 py-2 w-full border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
            </div>
            <button
                onClick={handleCreateClick}
                className="flex items-center justify-center bg-indigo-600 hover:bg-indigo-700 text-white font-bold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150"
            >
                <PlusCircle className="w-5 h-5 mr-2" />
                <span>Create Tag</span>
            </button>
        </div>
      </div>

      {renderContent()}

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