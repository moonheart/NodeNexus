import React, { useEffect } from 'react';
import { useForm, Controller } from 'react-hook-form';
import type { SubmitHandler } from 'react-hook-form';
import { X } from 'lucide-react';
import IconPicker from './IconPicker';
import * as tagService from '../services/tagService';
import type { Tag, CreateTagPayload, UpdateTagPayload } from '../types';

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
  const {
    register,
    handleSubmit,
    reset,
    control,
    watch, // Add watch
    setValue, // Add setValue
    formState: { errors, isSubmitting },
  } = useForm<TagFormInputs>({
    defaultValues: {
      name: '',
      color: '#ffffff',
      icon: '',
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
          icon: tag.icon || '',
          url: tag.url || '',
          is_visible: tag.isVisible,
        });
      } else {
        reset({
          name: '',
          color: '#ffffff',
          icon: '',
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
      } else {
        await tagService.createTag(payload as CreateTagPayload);
      }
      onTagSaved();
      onClose();
    } catch (err) {
      console.error('Failed to save tag:', err);
    }
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-slate-900/50 flex items-center justify-center z-50 transition-opacity duration-300">
      <div className="bg-white rounded-lg shadow-xl p-6 w-full max-w-md m-4 transform transition-all duration-300">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-xl font-bold text-slate-800">{tag ? 'Edit Tag' : 'Create New Tag'}</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 transition-colors">
            <X className="w-6 h-6" />
          </button>
        </div>

        <form onSubmit={handleSubmit(onSubmit)}>
          <div className="space-y-4">
            <div>
              <label htmlFor="tagName" className="block text-sm font-medium text-slate-700 mb-1">Name</label>
              <input
                type="text"
                id="tagName"
                {...register('name', { required: 'Name is required' })}
                className={`w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 ${errors.name ? 'border-red-500' : ''}`}
              />
              {errors.name && <p className="text-red-500 text-xs mt-1">{errors.name.message}</p>}
            </div>

            <div>
              <label htmlFor="tagColorText" className="block text-sm font-medium text-slate-700 mb-1">Color</label>
              <div className="flex items-center space-x-2">
                <input
                  type="color"
                  id="tagColorPicker"
                  value={watch('color') || '#ffffff'} // Ensure picker reflects form state, default to white if undefined
                  onChange={(e) => setValue('color', e.target.value, { shouldValidate: true, shouldDirty: true })}
                  className="h-10 w-16 px-1 py-1 border border-slate-300 rounded-md" // Adjusted width
                />
                <input
                  type="text"
                  id="tagColorText"
                  {...register('color', {
                    pattern: {
                      value: /^#([0-9A-Fa-f]{3,4}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$/i,
                      message: "Invalid hex color (e.g., #RRGGBB)"
                    }
                  })}
                  className={`flex-grow px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 ${errors.color ? 'border-red-500' : ''}`}
                  placeholder="#RRGGBB"
                />
              </div>
              {errors.color && <p className="text-red-500 text-xs mt-1">{errors.color.message}</p>}
            </div>

            <div>
              <label htmlFor="tagIcon" className="block text-sm font-medium text-slate-700 mb-1">Icon</label>
              <Controller
                name="icon"
                control={control}
                render={({ field }) => <IconPicker {...field} />}
              />
            </div>

            <div>
              <label htmlFor="tagUrl" className="block text-sm font-medium text-slate-700 mb-1">Associated URL (Optional)</label>
              <input
                type="url"
                id="tagUrl"
                placeholder="https://example.com"
                {...register('url')}
                className="w-full px-3 py-2 border border-slate-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
              />
            </div>

            <div className="flex items-center">
              <input
                type="checkbox"
                id="tagIsVisible"
                {...register('is_visible')}
                className="h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
              />
              <label htmlFor="tagIsVisible" className="ml-2 block text-sm text-slate-900">Visible in UI</label>
            </div>
          </div>

          <div className="mt-6 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="bg-slate-200 hover:bg-slate-300 text-slate-800 font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg shadow-sm transition-colors duration-150 disabled:bg-indigo-400 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Saving...' : (tag ? 'Update Tag' : 'Create Tag')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default TagEditModal;