import React, { useState, useEffect, useMemo } from 'react';
import { PlusCircle, Search } from 'lucide-react';
import toast from 'react-hot-toast';
import * as tagService from '../services/tagService';
import type { Tag, UpdateTagPayload } from '../types';
import { useServerListStore } from '../store/serverListStore';
import TagEditModal from '../components/TagEditModal';
import TagTableRow from '../components/TagTableRow';
import EmptyState from '../components/EmptyState';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const TagManagementPage: React.FC = () => {
  const tags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
  const [editingTag, setEditingTag] = useState<Tag | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [tagToDelete, setTagToDelete] = useState<Tag | null>(null);

  useEffect(() => {
    const loadInitialTags = async () => {
      try {
        await fetchAllTags();
      } catch (err) {
        toast.error('Failed to load tags.');
        console.error(err);
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

  const handleDeleteRequest = (tagId: number) => {
    const tag = tags.find(t => t.id === tagId);
    if (tag) {
      setTagToDelete(tag);
    }
  };

  const confirmDelete = () => {
    if (!tagToDelete) return;

    toast.promise(
      tagService.deleteTag(tagToDelete.id).then(() => {
        fetchAllTags();
        setTagToDelete(null);
      }),
      {
        loading: 'Deleting tag...',
        success: 'Tag deleted successfully!',
        error: 'Failed to delete tag.',
      }
    );
  };

  const handleToggleVisibility = async (tagId: number, isVisible: boolean) => {
    const originalTags = [...tags];
    const tagToUpdate = originalTags.find(t => t.id === tagId);
    if (!tagToUpdate) return;

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
      await fetchAllTags();
    } catch (error) {
      console.error("Failed to update visibility:", error);
      toast.error('Failed to update visibility.');
      useServerListStore.setState({ allTags: originalTags });
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
    if (!tags) {
        return <div className="text-center py-10">Loading tags...</div>;
    }
    if (tags.length === 0) {
      return (
        <EmptyState
          title="No Tags Found"
          message="Get started by creating your first tag."
          action={
            <Button onClick={handleCreateClick}>
              <PlusCircle className="w-4 h-4 mr-2" />
              Create Tag
            </Button>
          }
        />
      );
    }
    if (filteredTags.length === 0) {
      return <div className="text-center py-10 text-muted-foreground">No tags match your search.</div>;
    }
    return (
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>ID</TableHead>
            <TableHead className="text-center">Usage Count</TableHead>
            <TableHead>Associated URL</TableHead>
            <TableHead className="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {filteredTags.map((tag) => (
            <TagTableRow
              key={tag.id}
              tag={tag}
              onEdit={handleEditClick}
              onDelete={handleDeleteRequest}
              onToggleVisibility={handleToggleVisibility}
            />
          ))}
        </TableBody>
      </Table>
    );
  };

  return (
    <>
      <Card>
        <CardHeader className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
          <div>
            <CardTitle>Tag Management</CardTitle>
            <p className="text-muted-foreground text-sm mt-1">Create, edit, and manage all your tags.</p>
          </div>
          <div className="flex items-center gap-2 w-full sm:w-auto">
            <div className="relative w-full sm:w-64">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-muted-foreground" />
              <Input
                type="text"
                placeholder="Search tags..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-10"
              />
            </div>
            <Button onClick={handleCreateClick}>
              <PlusCircle className="w-5 h-5 mr-2" />
              <span>Create Tag</span>
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {renderContent()}
        </CardContent>
      </Card>

      <TagEditModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onTagSaved={handleTagSaved}
        tag={editingTag}
      />

      <AlertDialog open={!!tagToDelete} onOpenChange={() => setTagToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you absolutely sure?</AlertDialogTitle>
            <AlertDialogDescription>
              {tagToDelete && (tagToDelete.vpsCount ?? 0) > 0
                ? `This tag is used by ${tagToDelete.vpsCount} VPS(s). Deleting it will remove it from them. This action cannot be undone.`
                : 'This action cannot be undone. This will permanently delete the tag.'}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setTagToDelete(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>Continue</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
};

export default TagManagementPage;