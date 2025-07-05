import React, { useEffect, useState } from 'react';
import ServiceMonitorModal from '../components/ServiceMonitorModal';
import { createMonitor, updateMonitor, getMonitors, deleteMonitor } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorInput } from '../types';
import toast from 'react-hot-toast';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import ServiceMonitorTableRow from '@/components/ServiceMonitorTableRow';
import { Plus, RefreshCw } from 'lucide-react';
import { Skeleton } from '@/components/ui/skeleton';
import { useTranslation } from 'react-i18next';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Card, CardAction, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

const ServiceMonitoringPage: React.FC = () => {
  const { t } = useTranslation();
  const [monitors, setMonitors] = useState<ServiceMonitor[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingMonitor, setEditingMonitor] = useState<ServiceMonitor | null>(null);
  const [isAlertOpen, setIsAlertOpen] = useState(false);
  const [deletingMonitorId, setDeletingMonitorId] = useState<number | null>(null);

  const fetchMonitors = async () => {
    try {
      setIsLoading(true);
      const data = await getMonitors();
      setMonitors(data);
      setError(null);
    } catch (err) {
      setError(t('serviceMonitoring.notifications.fetchError'));
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchMonitors();
  }, []);

  const handleOpenCreateModal = () => {
    setEditingMonitor(null);
    setIsModalOpen(true);
  };

  const handleOpenEditModal = (monitor: ServiceMonitor) => {
    setEditingMonitor(monitor);
    setIsModalOpen(true);
  };

  const handleDeleteClick = (id: number) => {
    setDeletingMonitorId(id);
    setIsAlertOpen(true);
  };

  const confirmDelete = async () => {
    if (deletingMonitorId === null) return;
    try {
      await deleteMonitor(deletingMonitorId);
      toast.success(t('serviceMonitoring.notifications.deleteSuccess'));
      fetchMonitors(); // Refresh the list
    } catch (err) {
      toast.error(t('serviceMonitoring.notifications.deleteError'));
      console.error(err);
    } finally {
      setIsAlertOpen(false);
      setDeletingMonitorId(null);
    }
  };

  const handleSave = async (data: ServiceMonitorInput, id?: number) => {
    try {
      if (id) {
        await updateMonitor(id, data);
        toast.success(t('serviceMonitoring.notifications.saveSuccess_updated'));
      } else {
        await createMonitor(data);
        toast.success(t('serviceMonitoring.notifications.saveSuccess_created'));
      }
      setIsModalOpen(false);
      fetchMonitors(); // Refresh the list
    } catch (err) {
      toast.error(t('serviceMonitoring.notifications.saveError'));
      console.error(err);
    }
  };

  const SkeletonRow = () => (
    <TableRow>
      <TableCell><Skeleton className="h-5 w-24" /></TableCell>
      <TableCell><Skeleton className="h-5 w-16" /></TableCell>
      <TableCell><Skeleton className="h-5 w-32" /></TableCell>
      <TableCell><Skeleton className="h-5 w-20" /></TableCell>
      <TableCell><Skeleton className="h-5 w-40" /></TableCell>
      <TableCell><Skeleton className="h-5 w-28" /></TableCell>
      <TableCell className="text-right"><Skeleton className="h-8 w-20" /></TableCell>
    </TableRow>
  );

  return (
    <div className="container mx-auto p-4 md:p-6 lg:p-8">
      <Card>
        <CardHeader>
          <CardTitle>{t('serviceMonitoring.title')}</CardTitle>
          <CardAction>
            <Button onClick={fetchMonitors} variant="outline" className='mr-2'>
              <RefreshCw className="w-4 h-4" />
              {t('serviceMonitoring.refresh')}
            </Button>
            <Button onClick={handleOpenCreateModal}>
              <Plus className="w-4 h-4" />
              {t('serviceMonitoring.create')}
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <ServiceMonitorModal
            isOpen={isModalOpen}
            onClose={() => setIsModalOpen(false)}
            onSave={handleSave}
            monitorToEdit={editingMonitor}
          />

          <AlertDialog open={isAlertOpen} onOpenChange={setIsAlertOpen}>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>{t('serviceMonitoring.deleteDialog.title')}</AlertDialogTitle>
                <AlertDialogDescription>
                  {t('serviceMonitoring.deleteDialog.description')}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel onClick={() => setDeletingMonitorId(null)}>{t('serviceMonitoring.deleteDialog.cancel')}</AlertDialogCancel>
                <AlertDialogAction onClick={confirmDelete}>{t('serviceMonitoring.deleteDialog.confirm')}</AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

          {error && (
            <Alert variant="destructive">
              <AlertTitle>{t('common.errors.title')}</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t('serviceMonitoring.table.name')}</TableHead>
                <TableHead>{t('serviceMonitoring.table.type')}</TableHead>
                <TableHead>{t('serviceMonitoring.table.target')}</TableHead>
                <TableHead>{t('serviceMonitoring.table.frequency')}</TableHead>
                <TableHead>{t('serviceMonitoring.table.lastCheck')}</TableHead>
                <TableHead>{t('serviceMonitoring.table.statusMessage')}</TableHead>
                <TableHead className="text-right">{t('serviceMonitoring.table.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <>
                  <SkeletonRow />
                  <SkeletonRow />
                  <SkeletonRow />
                </>
              ) : !error && monitors.length > 0 ? (
                monitors.map((monitor) => (
                  <ServiceMonitorTableRow
                    key={monitor.id}
                    monitor={monitor}
                    onEdit={handleOpenEditModal}
                    onDelete={handleDeleteClick}
                  />
                ))
              ) : !error ? (
                <TableRow>
                  <TableCell colSpan={7}>
                    <div className="text-center py-16 border-2 border-dashed rounded-lg">
                      <h3 className="text-xl font-semibold text-muted-foreground">{t('serviceMonitoring.empty.title')}</h3>
                      <p className="text-muted-foreground mt-2">{t('serviceMonitoring.empty.description')}</p>
                      <Button className="mt-4" onClick={handleOpenCreateModal}>
                        <Plus className="w-4 h-4 mr-2" />
                        {t('serviceMonitoring.create')}
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
};

export default ServiceMonitoringPage;