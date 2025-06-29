import React, { useEffect } from 'react';
import { useForm, Controller } from 'react-hook-form';
import type { SubmitHandler } from 'react-hook-form';
import type { ChannelTemplate, ChannelResponse, CreateChannelRequest, UpdateChannelRequest } from '../types';
import DynamicForm from './DynamicForm';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { RefreshCwIcon as SpinnerIcon } from '@/components/Icons';

type FormInputs = {
  name: string;
  channelType: string;
  config: Record<string, unknown>;
};

interface NotificationChannelModalProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  onSubmit: (data: CreateChannelRequest | UpdateChannelRequest) => Promise<void>;
  templates: ChannelTemplate[];
  editingChannel?: ChannelResponse | null;
}

const NotificationChannelModal: React.FC<NotificationChannelModalProps> = ({
  isOpen,
  onOpenChange,
  onSubmit,
  templates,
  editingChannel,
}) => {
  const {
    control,
    handleSubmit,
    register,
    reset,
    watch,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<FormInputs>();

  const selectedChannelType = watch('channelType');
  const selectedTemplate = templates.find(t => t.channelType === selectedChannelType);

  useEffect(() => {
    if (isOpen) {
      if (editingChannel) {
        const template = templates.find(t => t.channelType === editingChannel.channelType);
        const initialConfig: Record<string, unknown> = {};
        if (template) {
          template.fields.forEach(field => {
            initialConfig[field.name] = editingChannel?.configParams?.[field.name] || '';
          });
        }
        reset({
          name: editingChannel.name,
          channelType: editingChannel.channelType,
          config: initialConfig,
        });
      } else {
        reset({
          name: '',
          channelType: '',
          config: {},
        });
      }
    }
  }, [isOpen, editingChannel, templates, reset]);

  useEffect(() => {
    // Reset config when template changes
    const newConfig: Record<string, unknown> = {};
    if (selectedTemplate) {
      selectedTemplate.fields.forEach(field => {
        newConfig[field.name] = ''; // Or a default value
      });
    }
    setValue('config', newConfig);
  }, [selectedTemplate, setValue]);

  const handleDynamicFormChange = (fieldName: string, value: unknown) => {
    setValue(`config.${fieldName}`, value);
  };

  const onFormSubmit: SubmitHandler<FormInputs> = async (data) => {
    if (!selectedTemplate) {
      // This should be caught by form validation, but as a safeguard
      console.error("No template selected");
      return;
    }
    
    const finalConfig: Record<string, unknown> = { type: data.channelType.toLowerCase() };
    selectedTemplate.fields.forEach(field => {
        if (data.config[field.name] !== undefined) {
            finalConfig[field.name] = data.config[field.name];
        }
    });

    const submissionData: CreateChannelRequest | UpdateChannelRequest = {
      name: data.name,
      channelType: data.channelType,
      config: finalConfig,
    };

    await onSubmit(submissionData);
    onOpenChange(false);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{editingChannel ? 'Edit' : 'Add New'} Notification Channel</DialogTitle>
          <DialogDescription>
            Select a channel type and fill in the required details.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(onFormSubmit)} className="space-y-6 py-4">
          <div className="space-y-2">
            <Label htmlFor="name">Channel Name</Label>
            <Input id="name" {...register('name', { required: 'Channel name is required' })} />
            {errors.name && <p className="text-sm text-destructive">{errors.name.message}</p>}
          </div>

          {!editingChannel && (
            <div className="space-y-2">
              <Label htmlFor="channelType">Channel Type</Label>
              <Controller
                name="channelType"
                control={control}
                rules={{ required: 'Please select a channel type' }}
                render={({ field }) => (
                  <Select onValueChange={field.onChange} defaultValue={field.value}>
                    <SelectTrigger>
                      <SelectValue placeholder="Select a type" />
                    </SelectTrigger>
                    <SelectContent>
                      {templates.map(template => (
                        <SelectItem key={template.channelType} value={template.channelType}>
                          {template.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              {errors.channelType && <p className="text-sm text-destructive">{errors.channelType.message}</p>}
            </div>
          )}

          {selectedTemplate ? (
            <Controller
              name="config"
              control={control}
              render={({ field }) => (
                <DynamicForm
                  fields={selectedTemplate.fields}
                  formData={field.value || {}}
                  onFormChange={handleDynamicFormChange}
                />
              )}
            />
          ) : !editingChannel ? (
            <Alert>
              <AlertDescription>Please select a channel type to see its configuration options.</AlertDescription>
            </Alert>
          ) : null}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
            <Button type="submit" disabled={isSubmitting || !selectedTemplate}>
              {isSubmitting && <SpinnerIcon className="mr-2 h-4 w-4 animate-spin" />}
              {isSubmitting ? 'Saving...' : (editingChannel ? 'Save Changes' : 'Create Channel')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default NotificationChannelModal;