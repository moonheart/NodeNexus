import React, { useEffect } from 'react';
import { useForm, Controller } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import toast from 'react-hot-toast';
import { useTranslation } from 'react-i18next';

import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Separator } from './ui/separator';

const formSchema = z.object({
  provider_name: z.string().min(1, "Provider Name is required"),
  client_id: z.string().min(1, "Client ID is required"),
  client_secret: z.string(),
  auth_url: z.string().url().optional().or(z.literal('')),
  token_url: z.string().url().optional().or(z.literal('')),
  user_info_url: z.string().url().optional().or(z.literal('')),
  scopes: z.string().optional().nullable(),
  icon_url: z.string().url().optional().or(z.literal('')),
  user_info_mapping: z.object({
    id_field: z.string().optional().nullable(),
    username_field: z.string().optional().nullable(),
  }).optional().nullable(),
  enabled: z.boolean(),
});

export type ProviderFormData = z.infer<typeof formSchema>;

export interface ProviderFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (providerData: Partial<ProviderFormData>) => Promise<void>;
  initialData?: Partial<ProviderFormData>;
}

const ProviderFormModal: React.FC<ProviderFormModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
  const { t } = useTranslation();

  const isEditing = !!initialData?.provider_name;

  const form = useForm<ProviderFormData>({
    resolver: zodResolver(formSchema.extend({
      provider_name: z.string().min(1, t('providerForm.validation.providerNameRequired')),
      client_id: z.string().min(1, t('providerForm.validation.clientIdRequired')),
    })),
    defaultValues: {
      provider_name: '',
      client_id: '',
      client_secret: '',
      auth_url: '',
      token_url: '',
      user_info_url: '',
      scopes: '',
      icon_url: '',
      user_info_mapping: {
        id_field: '',
        username_field: '',
      },
      enabled: true,
    },
  });

  const { handleSubmit, control, reset, formState: { isSubmitting, errors } } = form;

  useEffect(() => {
    if (isOpen) {
      if (isEditing && initialData) {
        reset(initialData);
      } else {
        reset({
          provider_name: '',
          client_id: '',
          client_secret: '',
          auth_url: '',
          token_url: '',
          user_info_url: '',
          scopes: '',
          icon_url: '',
          user_info_mapping: { id_field: 'id', username_field: 'name' },
          enabled: true,
        });
      }
    }
  }, [initialData, isOpen, isEditing, reset]);

  const handleFormSubmit = async (data: ProviderFormData) => {
    try {
      const payload = { ...data };
      if (isEditing && !payload.client_secret) {
        delete (payload as Partial<ProviderFormData>).client_secret;
      }
      await onSave(payload);
      toast.success(t('providerForm.notifications.saved', {
        status: isEditing ? t('providerForm.notifications.status.updated') : t('providerForm.notifications.status.created')
      }));
      onClose();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'An unknown error occurred.');
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{isEditing ? t('providerForm.title.edit') : t('providerForm.title.add')}</DialogTitle>
          <DialogDescription>
            {t('providerForm.description')}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit(handleFormSubmit)}>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4 py-4">
            {/* Left Column */}
            <div className="space-y-4">
              <div>
                <Label htmlFor="provider_name">{t('providerForm.labels.providerName')} <span className="text-destructive">*</span></Label>
                <Controller name="provider_name" control={control} render={({ field }) => <Input id="provider_name" {...field} disabled={isEditing} />} />
                {errors.provider_name && <p className="text-sm text-destructive mt-1">{errors.provider_name.message}</p>}
              </div>
              <div>
                <Label htmlFor="client_id">{t('providerForm.labels.clientId')} <span className="text-destructive">*</span></Label>
                <Controller name="client_id" control={control} render={({ field }) => <Input id="client_id" {...field} />} />
                {errors.client_id && <p className="text-sm text-destructive mt-1">{errors.client_id.message}</p>}
              </div>
              <div>
                <Label htmlFor="client_secret">{t('providerForm.labels.clientSecret')} {!isEditing && <span className="text-destructive">*</span>}</Label>
                <Controller name="client_secret" control={control} render={({ field }) => <Input id="client_secret" type="password" placeholder={isEditing ? t('providerForm.placeholders.unchanged') : ''} {...field} />} />
              </div>
              <div>
                <Label htmlFor="scopes">{t('providerForm.labels.scopes')}</Label>
                <Controller name="scopes" control={control} render={({ field }) => <Input id="scopes" {...field} value={field.value || ''} />} />
              </div>
              <div>
                <Label htmlFor="icon_url">{t('providerForm.labels.iconUrl')}</Label>
                <Controller name="icon_url" control={control} render={({ field }) => <Input id="icon_url" {...field} value={field.value || ''} />} />
              </div>
            </div>

            {/* Right Column */}
            <div className="space-y-4">
              <div>
                <Label htmlFor="auth_url">{t('providerForm.labels.authUrl')}</Label>
                <Controller name="auth_url" control={control} render={({ field }) => <Input id="auth_url" {...field} value={field.value || ''} />} />
              </div>
              <div>
                <Label htmlFor="token_url">{t('providerForm.labels.tokenUrl')}</Label>
                <Controller name="token_url" control={control} render={({ field }) => <Input id="token_url" {...field} value={field.value || ''} />} />
              </div>
              <div>
                <Label htmlFor="user_info_url">{t('providerForm.labels.userInfoUrl')}</Label>
                <Controller name="user_info_url" control={control} render={({ field }) => <Input id="user_info_url" {...field} value={field.value || ''} />} />
              </div>
            </div>
          </div>

          <Separator className="my-4" />

          {/* User Info Mapping Section */}
          <div>
            <h4 className="text-md font-medium text-slate-900">{t('providerForm.labels.userInfoMapping')}</h4>
            <p className="text-sm text-muted-foreground mt-1">
              {t('providerForm.labels.userInfoMappingDesc')}
            </p>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4 mt-4">
              <div>
                <Label htmlFor="id_field">{t('providerForm.labels.idField')}</Label>
                <Controller name="user_info_mapping.id_field" control={control} render={({ field }) => <Input id="id_field" placeholder={t('providerForm.placeholders.idField')} {...field} value={field.value || ''} />} />
              </div>
              <div>
                <Label htmlFor="username_field">{t('providerForm.labels.usernameField')}</Label>
                <Controller name="user_info_mapping.username_field" control={control} render={({ field }) => <Input id="username_field" placeholder={t('providerForm.placeholders.usernameField')} {...field} value={field.value || ''} />} />
              </div>
            </div>
          </div>

          <DialogFooter className="pt-8">
            <div className="flex items-center mr-auto">
                <Controller name="enabled" control={control} render={({ field }) => <Switch id="enabled" checked={field.value} onCheckedChange={field.onChange} />} />
                <Label htmlFor="enabled" className="ml-2">{t('providerForm.labels.enabled')}</Label>
            </div>
            <Button type="button" variant="outline" onClick={onClose}>{t('common.actions.cancel')}</Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t('common.status.saving') : t('common.actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default ProviderFormModal;