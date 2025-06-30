import React, { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useForm, Controller } from 'react-hook-form';
import type { SubmitHandler } from 'react-hook-form';
import IconPicker from './IconPicker';
import * as tagService from '../services/tagService';
import type { Tag, CreateTagPayload, UpdateTagPayload } from '../types';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { toast } from 'react-hot-toast';

type TagFormInputs = {
  name: string;
  color: string;
  icon: string;
  url: string;
  is_visible: boolean;
};

interface TagEditModalProps {
  isOpen: boolean;
  onClose: () => void;
  onTagSaved: () => void;
  tag: Tag | null;
}

const TagEditModal: React.FC<TagEditModalProps> = ({ isOpen, onClose, onTagSaved, tag }) => {
  const { t } = useTranslation();
  const {
    register,
    handleSubmit,
    reset,
    control,
    watch,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<TagFormInputs>({
    defaultValues: {
      name: '',
      color: '#4f46e5', // A better default color
      icon: 'Tag',
      url: '',
      is_visible: true,
    },
  });

  useEffect(() => {
    if (isOpen) {
      if (tag) {
        reset({
          name: tag.name,
          color: tag.color,
          icon: tag.icon || 'Tag',
          url: tag.url || '',
          is_visible: tag.isVisible,
        });
      } else {
        reset({
          name: '',
          color: '#4f46e5',
          icon: 'Tag',
          url: '',
          is_visible: true,
        });
      }
    }
  }, [isOpen, tag, reset]);

  const onSubmit: SubmitHandler<TagFormInputs> = async (data) => {
    try {
      const payload = {
        name: data.name,
        color: data.color,
        icon: data.icon || undefined,
        url: data.url || undefined,
        is_visible: data.is_visible,
      };

      if (tag) {
        await tagService.updateTag(tag.id, payload as UpdateTagPayload);
        toast.success(t('common.notifications.updated'));
      } else {
        await tagService.createTag(payload as CreateTagPayload);
        toast.success(t('common.notifications.created'));
      }
      onTagSaved();
      onClose();
    } catch (err) {
      console.error('Failed to save tag:', err);
      toast.error(t('common.notifications.saveFailed'));
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{tag ? t('common.form.editTitle') : t('common.form.createTitle')}</DialogTitle>
          <DialogDescription>
            {tag ? t('tagManagement.form.editDescription') : t('tagManagement.form.createDescription')}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)}>
          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="name" className="text-right">
                {t('common.labels.name')}
              </Label>
              <Input
                id="name"
                {...register('name', { required: t('common.errors.validation.nameRequired') })}
                className={`col-span-3 ${errors.name ? 'border-destructive' : ''}`}
              />
              {errors.name && <p className="col-span-4 text-destructive text-xs text-right">{errors.name.message}</p>}
            </div>

            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="color" className="text-right">
                {t('common.labels.color')}
              </Label>
              <div className="col-span-3 flex items-center gap-2">
                <Input
                  id="color-picker"
                  type="color"
                  value={watch('color') || '#4f46e5'}
                  onChange={(e) => setValue('color', e.target.value, { shouldValidate: true, shouldDirty: true })}
                  className="p-1 h-10 w-14"
                />
                <Input
                  id="color"
                  {...register('color', {
                    pattern: {
                      value: /^#([0-9A-Fa-f]{6})$/i,
                      message: t('common.errors.validation.invalidColor')
                    }
                  })}
                  className={`flex-grow ${errors.color ? 'border-destructive' : ''}`}
                  placeholder="#RRGGBB"
                />
              </div>
              {errors.color && <p className="col-span-4 text-destructive text-xs text-right">{errors.color.message}</p>}
            </div>

            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="icon" className="text-right">
                {t('common.labels.icon')}
              </Label>
              <div className="col-span-3">
                <IconPicker
                  value={watch('icon')}
                  onChange={(name) => setValue('icon', name, { shouldValidate: true, shouldDirty: true })}
                />
              </div>
            </div>

            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="url" className="text-right">
                {t('common.labels.url')}
              </Label>
              <Input
                id="url"
                type="url"
                placeholder="https://example.com"
                {...register('url')}
                className="col-span-3"
              />
            </div>

            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="is_visible" className="text-right">
                {t('common.labels.visible')}
              </Label>
              <div className="col-span-3">
                <Controller
                  name="is_visible"
                  control={control}
                  render={({ field }) => (
                    <Switch
                      id="is_visible"
                      checked={field.value}
                      onCheckedChange={field.onChange}
                    />
                  )}
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>{t('common.actions.cancel')}</Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t('common.status.saving') : (tag ? t('common.actions.update') : t('common.actions.create'))}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default TagEditModal;