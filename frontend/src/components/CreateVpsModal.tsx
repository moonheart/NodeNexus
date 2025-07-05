import React, { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { createVps } from '../services/vpsService';
import type { Vps } from '../types';
import axios from 'axios';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from 'react-hot-toast';

interface CreateVpsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onVpsCreated?: (newVps: Vps) => void;
}

const CreateVpsModal: React.FC<CreateVpsModalProps> = ({ isOpen, onClose, onVpsCreated }) => {
  const { t } = useTranslation();
  const [vpsName, setVpsName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setVpsName('');
      setError(null);
      setIsLoading(false);
    }
  }, [isOpen]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);

    if (!vpsName.trim()) {
      setError(t('serverManagement.modals.create.nameRequired'));
      return;
    }

    setIsLoading(true);
    try {
      const payload: import('../services/vpsService').CreateVpsPayload = {
        name: vpsName.trim(),
      };
      const newVps = await createVps(payload);
      toast.success(t('serverManagement.modals.create.createSuccess', { name: newVps.name }));
      
      if (onVpsCreated) {
        onVpsCreated(newVps);
      }
      onClose(); // Close modal on success
    } catch (err: unknown) {
      console.error('Failed to create VPS:', err);
      let errorMessage = t('serverManagement.modals.edit.updateFailed');
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        errorMessage = err.response.data.error;
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }
      setError(errorMessage);
      toast.error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{t('serverManagement.modals.create.title')}</DialogTitle>
          <DialogDescription>
            {t('serverManagement.modals.create.description')}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="vpsNameModal" className="text-right">
                {t('serverManagement.modals.create.nameLabel')}
              </Label>
              <Input
                id="vpsNameModal"
                value={vpsName}
                onChange={(e) => setVpsName(e.target.value)}
                placeholder={t('serverManagement.modals.create.namePlaceholder')}
                required
                className={`col-span-3 ${error ? 'border-destructive' : ''}`}
              />
            </div>
            {error && <p className="col-start-2 col-span-3 text-destructive text-sm">{error}</p>}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>{t('common.actions.cancel')}</Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? t('serverManagement.modals.create.creating') : t('serverManagement.modals.create.createButton')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default CreateVpsModal;