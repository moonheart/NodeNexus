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

const NotificationsPage: React.FC = () => {
    const [channels, setChannels] = useState<ChannelResponse[]>([]);
    const [templates, setTemplates] = useState<ChannelTemplate[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [currentEditingChannel, setCurrentEditingChannel] = useState<ChannelResponse | null>(null);

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
            setError('Failed to load initial data.');
            console.error(err);
            toast.error('Failed to load data.');
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

    const handleModalClose = () => {
        setIsModalOpen(false);
        setCurrentEditingChannel(null);
    };

    const handleModalSubmit = async (data: CreateChannelRequest | UpdateChannelRequest) => {
        try {
            if (currentEditingChannel && 'id' in currentEditingChannel) {
                 // Ensure data is UpdateChannelRequest; might need type assertion or check
                await updateChannel(currentEditingChannel.id, data as UpdateChannelRequest);
                toast.success('Channel updated successfully!');
            } else {
                await createChannel(data as CreateChannelRequest);
                toast.success('Channel created successfully!');
            }
            fetchAllData(); // Refresh list
        } catch (err) {
            console.error('Failed to save channel:', err);
            toast.error('Failed to save channel.');
            // setError will be handled by the modal itself if needed, or pass error back up
            throw err; // Re-throw to let modal handle its own error state if desired
        }
    };

    const handleDeleteChannel = async (id: number) => {
        if (window.confirm('Are you sure you want to delete this channel?')) {
            try {
                await deleteChannel(id);
                toast.success('Channel deleted successfully!');
                fetchAllData(); // Refresh list
            } catch (err) {
                console.error('Failed to delete channel:', err);
                toast.error('Failed to delete channel.');
            }
        }
    };

    const handleTestChannel = async (id: number) => {
        try {
            await testChannel(id, 'This is a test message.');
            toast.success('Test message sent successfully!');
        } catch (err) {
            console.error('Failed to send test message:', err);
            toast.error('Failed to send test message.');
        }
    };

    if (isLoading) {
        return <div className="container mx-auto p-4">Loading notification settings...</div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error: {error}</div>;
    }

    return (
        <div className="container mx-auto p-4">
            <div className="flex justify-between items-center mb-6">
                <h1 className="text-2xl font-bold">Notification Channels</h1>
                <button
                    onClick={handleOpenCreateModal}
                    className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700"
                >
                    Add New Channel
                </button>
            </div>

            {channels.length === 0 && !isLoading ? (
                 <div className="text-center py-10">
                    <p className="text-gray-500">No notification channels configured yet.</p>
                    <button
                        onClick={handleOpenCreateModal}
                        className="mt-4 bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700"
                    >
                        Add Your First Channel
                    </button>
                </div>
            ) : (
                <div className="bg-white p-6 rounded-lg shadow-md">
                    <h2 className="text-xl font-semibold mb-4">Configured Channels</h2>
                    <ul className="divide-y divide-gray-200">
                        {channels.map(channel => (
                            <li key={channel.id} className="py-4 flex justify-between items-center">
                                <div>
                                    <span className="font-semibold text-lg">{channel.name}</span>
                                    <span className="block text-sm text-gray-500 capitalize">{channel.channelType}</span>
                                </div>
                                <div className="space-x-2">
                                    <button
                                        onClick={() => handleOpenEditModal(channel)}
                                        className="text-indigo-600 hover:text-indigo-900 font-medium"
                                    >
                                        Edit
                                    </button>
                                    <button
                                        onClick={() => handleTestChannel(channel.id)}
                                        className="text-green-600 hover:text-green-900 font-medium"
                                    >
                                        Test
                                    </button>
                                    <button
                                        onClick={() => handleDeleteChannel(channel.id)}
                                        className="text-red-600 hover:text-red-900 font-medium"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </li>
                        ))}
                    </ul>
                </div>
            )}

            {isModalOpen && (
                <NotificationChannelModal
                    isOpen={isModalOpen}
                    onClose={handleModalClose}
                    onSubmit={handleModalSubmit}
                    templates={templates}
                    editingChannel={currentEditingChannel}
                />
            )}
        </div>
    );
};

export default NotificationsPage;