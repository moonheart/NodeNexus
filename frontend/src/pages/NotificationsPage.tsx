import React, { useState, useEffect, useCallback } from 'react';
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
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
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
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { MoreHorizontal, PlusCircle, Send, Trash2, Edit } from 'lucide-react';
import { RefreshCwIcon as SpinnerIcon } from '@/components/Icons';
import EmptyState from '@/components/EmptyState';

const NotificationsPage: React.FC = () => {
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
            const errorMessage = err instanceof Error ? err.message : 'Failed to load data.';
            setError(errorMessage);
            console.error(err);
            toast.error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, []);

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
        const toastId = toast.loading(isEditing ? 'Updating channel...' : 'Creating channel...');
        try {
            if (isEditing) {
                await updateChannel(currentEditingChannel.id, data as UpdateChannelRequest);
            } else {
                await createChannel(data as CreateChannelRequest);
            }
            toast.success(`Channel ${isEditing ? 'updated' : 'created'} successfully!`, { id: toastId });
            fetchAllData();
        } catch (err) {
            console.error('Failed to save channel:', err);
            toast.error(`Failed to ${isEditing ? 'update' : 'create'} channel.`, { id: toastId });
            throw err;
        }
    };

    const handleDeleteClick = (id: number) => {
        setDeletingChannelId(id);
        setIsDeleteAlertOpen(true);
    };

    const confirmDeleteChannel = async () => {
        if (deletingChannelId === null) return;
        const toastId = toast.loading('Deleting channel...');
        try {
            await deleteChannel(deletingChannelId);
            toast.success('Channel deleted successfully!', { id: toastId });
            fetchAllData();
        } catch (err) {
            console.error('Failed to delete channel:', err);
            toast.error('Failed to delete channel.', { id: toastId });
        } finally {
            setIsDeleteAlertOpen(false);
            setDeletingChannelId(null);
        }
    };

    const handleTestChannel = async (id: number) => {
        setTestingChannelId(id);
        const toastId = toast.loading('Sending test message...');
        try {
            await testChannel(id, 'This is a test message from the dashboard.');
            toast.success('Test message sent successfully!', { id: toastId });
        } catch (err) {
            console.error('Failed to send test message:', err);
            toast.error('Failed to send test message.', { id: toastId });
        } finally {
            setTestingChannelId(null);
        }
    };

    if (isLoading) {
        return <div className="flex items-center justify-center h-full"><SpinnerIcon className="h-8 w-8 animate-spin" /></div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-destructive">Error: {error}</div>;
    }

    return (
        <div className="space-y-6">
            <Card>
                <CardHeader className="flex flex-row items-center justify-between">
                    <div>
                        <CardTitle>Notification Channels</CardTitle>
                        <CardDescription>Manage channels to receive alerts and notifications.</CardDescription>
                    </div>
                    <Button onClick={handleOpenCreateModal}>
                        <PlusCircle size={18} className="mr-2" /> Add New Channel
                    </Button>
                </CardHeader>
                <CardContent>
                    {channels.length === 0 ? (
                        <EmptyState
                            title="No Notification Channels"
                            message="Get started by creating your first notification channel."
                            action={<Button onClick={handleOpenCreateModal}><PlusCircle size={18} className="mr-2" /> Add New Channel</Button>}
                        />
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Name</TableHead>
                                    <TableHead>Type</TableHead>
                                    <TableHead className="text-right">Actions</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {channels.map(channel => (
                                    <TableRow key={channel.id}>
                                        <TableCell className="font-medium">{channel.name}</TableCell>
                                        <TableCell className="capitalize text-muted-foreground">{channel.channelType}</TableCell>
                                        <TableCell className="text-right">
                                            <DropdownMenu>
                                                <DropdownMenuTrigger asChild>
                                                    <Button variant="ghost" className="h-8 w-8 p-0">
                                                        <span className="sr-only">Open menu</span>
                                                        <MoreHorizontal className="h-4 w-4" />
                                                    </Button>
                                                </DropdownMenuTrigger>
                                                <DropdownMenuContent align="end">
                                                    <DropdownMenuItem onClick={() => handleOpenEditModal(channel)}>
                                                        <Edit className="mr-2 h-4 w-4" />
                                                        <span>Edit</span>
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => handleTestChannel(channel.id)} disabled={testingChannelId === channel.id}>
                                                        {testingChannelId === channel.id ? <SpinnerIcon className="mr-2 h-4 w-4 animate-spin" /> : <Send className="mr-2 h-4 w-4" />}
                                                        <span>Test</span>
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => handleDeleteClick(channel.id)} className="text-destructive">
                                                        <Trash2 className="mr-2 h-4 w-4" />
                                                        <span>Delete</span>
                                                    </DropdownMenuItem>
                                                </DropdownMenuContent>
                                            </DropdownMenu>
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    )}
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
                        <AlertDialogTitle>Are you sure?</AlertDialogTitle>
                        <AlertDialogDescription>
                            This action cannot be undone. This will permanently delete the notification channel.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setDeletingChannelId(null)}>Cancel</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmDeleteChannel}>Delete</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default NotificationsPage;