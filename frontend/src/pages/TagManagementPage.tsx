import React, { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
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
        toast.error(t('tagManagement.notifications.fetchFailed'));
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
        loading: t('tagManagement.notifications.deleting'),
        success: t('tagManagement.notifications.deleted'),
        error: t('tagManagement.notifications.deleteFailed'),
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
      toast.success(t('tagManagement.notifications.visibilityUpdated'));
      await fetchAllTags();
    } catch (error) {
      console.error("Failed to update visibility:", error);
      toast.error(t('tagManagement.notifications.visibilityUpdateFailed'));
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
        return <div className="text-center py-10">{t('tagManagement.loading')}</div>;
    }
    if (tags.length === 0) {
      return (
        <EmptyState
          title={t('tagManagement.empty.title')}
          message={t('tagManagement.empty.description')}
          action={
            <Button onClick={handleCreateClick}>
              <PlusCircle className="w-4 h-4 mr-2" />
              {t('tagManagement.empty.newButton')}
            </Button>
          }
        />
      );
    }
    if (filteredTags.length === 0) {
      return <div className="text-center py-10 text-muted-foreground">{t('tagManagement.noMatch')}</div>;
    }
    return (
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t('tagManagement.table.name')}</TableHead>
            <TableHead>{t('tagManagement.table.id')}</TableHead>
            <TableHead className="text-center">{t('tagManagement.table.usageCount')}</TableHead>
            <TableHead>{t('tagManagement.table.associatedUrl')}</TableHead>
            <TableHead className="text-right">{t('tagManagement.table.actions')}</TableHead>
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
            <CardTitle>{t('tagManagement.title')}</CardTitle>
            <p className="text-muted-foreground text-sm mt-1">{t('tagManagement.description')}</p>
          </div>
          <div className="flex items-center gap-2 w-full sm:w-auto">
            <div className="relative w-full sm:w-64">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-muted-foreground" />
              <Input
                type="text"
                placeholder={t('tagManagement.searchPlaceholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-10"
              />
            </div>
            <Button onClick={handleCreateClick}>
              <PlusCircle className="w-5 h-5 mr-2" />
              <span>{t('tagManagement.addNew')}</span>
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
            <AlertDialogTitle>{t('tagManagement.deleteDialog.title')}</AlertDialogTitle>
            <AlertDialogDescription>
              {tagToDelete && (tagToDelete.vpsCount ?? 0) > 0
                ? t('tagManagement.deleteDialog.descriptionWithCount', { count: tagToDelete.vpsCount })
                : t('tagManagement.deleteDialog.description')}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setTagToDelete(null)}>{t('buttons.cancel')}</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete}>{t('tagManagement.deleteDialog.confirm')}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
};

export default TagManagementPage;