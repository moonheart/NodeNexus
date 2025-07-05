import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
    getChannelTemplates,
    getAllChannels,
    createChannel,
    updateChannel,
    deleteChannel,
    testChannel,
} from '../services/notificationService';
import type { ChannelTemplate, ChannelResponse, CreateChannelRequest, UpdateChannelRequest } from '../types';
import NotificationChannelModal from '../components/NotificationChannelModal';
import toast from 'react-hot-toast';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardAction } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
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
import { Skeleton } from "@/components/ui/skeleton";
import { PlusCircle, Send, Trash2, Edit } from 'lucide-react';
import { RefreshCwIcon as SpinnerIcon } from '@/components/Icons';
import EmptyState from '@/components/EmptyState';

const NotificationsPage: React.FC = () => {
    const { t } = useTranslation();
    const [channels, setChannels] = useState<ChannelResponse[]>([]);
    const [templates, setTemplates] = useState<ChannelTemplate[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [currentEditingChannel, setCurrentEditingChannel] = useState<ChannelResponse | null>(null);
    const [isDeleteAlertOpen, setIsDeleteAlertOpen] = useState(false);
    const [deletingChannelId, setDeletingChannelId] = useState<number | null>(null);
    const [testingChannelId, setTestingChannelId] = useState<number | null>(null);

    const fetchAllData = useCallback(async () => {
        setIsLoading(true);
        try {
            const [templatesData, channelsData] = await Promise.all([
                getChannelTemplates(),
                getAllChannels(),
            ]);
            setTemplates(templatesData);
            setChannels(channelsData);
            setError(null);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : t('common.notifications.fetchFailed');
            setError(errorMessage);
            console.error(err);
            toast.error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [t]);

    useEffect(() => {
        fetchAllData();
    }, [fetchAllData]);

    const handleOpenCreateModal = () => {
        setCurrentEditingChannel(null);
        setIsModalOpen(true);
    };

    const handleOpenEditModal = (channel: ChannelResponse) => {
        setCurrentEditingChannel(channel);
        setIsModalOpen(true);
    };

    const handleModalSubmit = async (data: CreateChannelRequest | UpdateChannelRequest) => {
        const isEditing = !!currentEditingChannel;
        const toastId = toast.loading(isEditing ? t('notificationsPage.status.updating') : t('notificationsPage.status.creating'));
        try {
            if (isEditing) {
                await updateChannel(currentEditingChannel.id, data as UpdateChannelRequest);
            } else {
                await createChannel(data as CreateChannelRequest);
            }
            toast.success(t('notificationsPage.notifications.saveSuccess'), { id: toastId });
            fetchAllData();
        } catch (err) {
            console.error('Failed to save channel:', err);
            toast.error(t('notificationsPage.notifications.saveFailed'), { id: toastId });
            throw err;
        }
    };

    const handleDeleteClick = (id: number) => {
        setDeletingChannelId(id);
        setIsDeleteAlertOpen(true);
    };

    const confirmDeleteChannel = async () => {
        if (deletingChannelId === null) return;
        const toastId = toast.loading(t('notificationsPage.status.deleting'));
        try {
            await deleteChannel(deletingChannelId);
            toast.success(t('notificationsPage.notifications.deleteSuccess'), { id: toastId });
            fetchAllData();
        } catch (err) {
            console.error('Failed to delete channel:', err);
            toast.error(t('notificationsPage.notifications.deleteFailed'), { id: toastId });
        } finally {
            setIsDeleteAlertOpen(false);
            setDeletingChannelId(null);
        }
    };

    const handleTestChannel = async (id: number) => {
        setTestingChannelId(id);
        const toastId = toast.loading(t('notificationsPage.status.testing'));
        try {
            await testChannel(id, 'This is a test message from the dashboard.');
            toast.success(t('notificationsPage.notifications.testSuccess'), { id: toastId });
        } catch (err) {
            console.error('Failed to send test message:', err);
            toast.error(t('notificationsPage.notifications.testFailed'), { id: toastId });
        } finally {
            setTestingChannelId(null);
        }
    };

    if (error) {
        return <div className="container mx-auto p-4 text-destructive">{t('common.notifications.error', { error: error })}</div>;
    }

    const renderTableContent = () => {
        if (isLoading) {
            return (
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead>{t('notificationsPage.table.name')}</TableHead>
                            <TableHead>{t('notificationsPage.table.type')}</TableHead>
                            <TableHead className="text-right">{t('notificationsPage.table.actions')}</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {Array.from({ length: 3 }).map((_, index) => (
                            <TableRow key={index}>
                                <TableCell><Skeleton className="h-5 w-32" /></TableCell>
                                <TableCell><Skeleton className="h-5 w-20" /></TableCell>
                                <TableCell className="text-right space-x-1">
                                    <Skeleton className="h-8 w-8 inline-block" />
                                    <Skeleton className="h-8 w-8 inline-block" />
                                    <Skeleton className="h-8 w-8 inline-block" />
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            );
        }

        if (channels.length === 0) {
            return (
                <EmptyState
                    title={t('notificationsPage.empty.title')}
                    message={t('notificationsPage.empty.message')}
                    action={<Button onClick={handleOpenCreateModal}><PlusCircle size={18} className="mr-2" /> {t('notificationsPage.addNew')}</Button>}
                />
            );
        }

        return (
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead>{t('notificationsPage.table.name')}</TableHead>
                        <TableHead>{t('notificationsPage.table.type')}</TableHead>
                        <TableHead className="text-right">{t('notificationsPage.table.actions')}</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {channels.map(channel => (
                        <TableRow key={channel.id}>
                            <TableCell className="font-medium">{channel.name}</TableCell>
                            <TableCell className="capitalize text-muted-foreground">{channel.channelType}</TableCell>
                            <TableCell className="text-right space-x-1">
                                <Button variant="ghost" size="icon" onClick={() => handleOpenEditModal(channel)}>
                                    <Edit className="h-4 w-4" />
                                </Button>
                                <Button variant="ghost" size="icon" onClick={() => handleTestChannel(channel.id)} disabled={testingChannelId === channel.id}>
                                    {testingChannelId === channel.id ? <SpinnerIcon className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
                                </Button>
                                <Button variant="ghost" size="icon" onClick={() => handleDeleteClick(channel.id)}>
                                    <Trash2 className="h-4 w-4 text-destructive" />
                                </Button>
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        );
    };

    return (
        <div className="space-y-6">
            <Card>
                <CardHeader>
                    <CardTitle>{t('notificationsPage.title')}</CardTitle>
                    <CardDescription>{t('notificationsPage.description')}</CardDescription>
                    <CardAction>
                        <Button onClick={handleOpenCreateModal}>
                            <PlusCircle size={18} className="mr-2" /> {t('notificationsPage.addNew')}
                        </Button>
                    </CardAction>
                </CardHeader>
                <CardContent>
                    {renderTableContent()}
                </CardContent>
            </Card>

            <NotificationChannelModal
                isOpen={isModalOpen}
                onOpenChange={setIsModalOpen}
                onSubmit={handleModalSubmit}
                templates={templates}
                editingChannel={currentEditingChannel}
            />

            <AlertDialog open={isDeleteAlertOpen} onOpenChange={setIsDeleteAlertOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.dialogs.delete.title')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('notificationsPage.deleteDialog.description')}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setDeletingChannelId(null)}>{t('common.actions.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmDeleteChannel}>{t('common.actions.delete')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default NotificationsPage;