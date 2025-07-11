import React, { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useServerListStore } from '../store/serverListStore';
import * as tagService from '../services/tagService';
import { toast } from 'react-hot-toast';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChevronDown } from 'lucide-react';
import { Badge } from './ui/badge';

interface BulkEditTagsModalProps {
  isOpen: boolean;
  onClose: () => void;
  vpsIds: number[];
  onTagsUpdated: () => void;
}

const BulkEditTagsModal: React.FC<BulkEditTagsModalProps> = ({ isOpen, onClose, vpsIds, onTagsUpdated }) => {
  const { t } = useTranslation();
  const [tagsToAdd, setTagsToAdd] = useState<number[]>([]);
  const [tagsToRemove, setTagsToRemove] = useState<number[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const allTags = useServerListStore((state) => state.allTags);
  const fetchAllTags = useServerListStore((state) => state.fetchAllTags);

  useEffect(() => {
    if (isOpen) {
      fetchAllTags();
      setTagsToAdd([]);
      setTagsToRemove([]);
      setIsLoading(false);
    }
  }, [isOpen, fetchAllTags]);

  const addOptions = useMemo(() => allTags.filter(tag => !tagsToRemove.includes(tag.id)), [allTags, tagsToRemove]);
  const removeOptions = useMemo(() => allTags.filter(tag => !tagsToAdd.includes(tag.id)), [allTags, tagsToAdd]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    
    try {
      await tagService.bulkUpdateVpsTags({
        vpsIds: vpsIds,
        addTagIds: tagsToAdd,
        removeTagIds: tagsToRemove,
      });
      toast.success(t('serverManagement.modals.bulkEditTags.updateSuccess'));
      onTagsUpdated();
      onClose();
    } catch (err) {
      console.error('Failed to bulk update tags:', err);
      toast.error(t('serverManagement.modals.bulkEditTags.updateFailed'));
    } finally {
      setIsLoading(false);
    }
  };

  const MultiSelectPopover = ({
    label,
    options,
    selected,
    onSelectedChange,
  }: {
    label: string;
    options: typeof allTags;
    selected: number[];
    onSelectedChange: (selected: number[]) => void;
  }) => (
    <div>
      <Label className="block text-sm font-medium text-slate-700 mb-1">{label}</Label>
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="outline" className="w-full justify-between">
            <span className="truncate">
              {selected.length > 0
                ? t('serverManagement.modals.bulkEditTags.selected', { count: selected.length })
                : t('serverManagement.modals.bulkEditTags.placeholder')}
            </span>
            <ChevronDown className="h-4 w-4 ml-2" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[--radix-popover-trigger-width] p-0">
          <ScrollArea className="h-48">
            <div className="p-4 space-y-2">
              {options.map(tag => (
                <div key={tag.id} className="flex items-center space-x-2">
                  <Checkbox
                    id={`tag-${tag.id}-${label}`}
                    checked={selected.includes(tag.id)}
                    onCheckedChange={(checked) => {
                      const newSelected = checked
                        ? [...selected, tag.id]
                        : selected.filter(id => id !== tag.id);
                      onSelectedChange(newSelected);
                    }}
                  />
                  <Label htmlFor={`tag-${tag.id}-${label}`} className="flex-grow">
                    <Badge style={{ backgroundColor: tag.color, color: '#fff' }}>{tag.name}</Badge>
                  </Label>
                </div>
              ))}
            </div>
          </ScrollArea>
        </PopoverContent>
      </Popover>
    </div>
  );

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t('serverManagement.modals.bulkEditTags.title')}</DialogTitle>
          <DialogDescription>
            {t('serverManagement.modals.bulkEditTags.description', { count: vpsIds.length })}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <MultiSelectPopover
              label={t('serverManagement.modals.bulkEditTags.tagsToAdd')}
              options={addOptions}
              selected={tagsToAdd}
              onSelectedChange={setTagsToAdd}
            />
            <MultiSelectPopover
              label={t('serverManagement.modals.bulkEditTags.tagsToRemove')}
              options={removeOptions}
              selected={tagsToRemove}
              onSelectedChange={setTagsToRemove}
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              {t('common.actions.cancel')}
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? t('common.status.saving') : t('common.actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default BulkEditTagsModal;