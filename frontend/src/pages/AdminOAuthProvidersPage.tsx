// frontend/src/pages/AdminOAuthProvidersPage.tsx

import React, { useState, useEffect } from 'react';
import { useAuthStore } from '../store/authStore';
import toast from 'react-hot-toast';
import ProviderFormModal, { type ProviderFormData } from '../components/ProviderFormModal';

// This type should match the `AdminProviderInfo` struct from the backend
export interface OAuthProvider {
    provider_name: string;
    client_id: string;
    client_secret: string;
    auth_url: string;
    token_url: string;
    user_info_url: string;
    scopes: string | null;
    icon_url: string | null;
    user_info_mapping: {
        id_field: string;
        username_field: string;
    } | null;
    enabled: boolean;
}

const AdminOAuthProvidersPage: React.FC = () => {
    const [providers, setProviders] = useState<OAuthProvider[]>([]);
    const [loading, setLoading] = useState(true);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [editingProvider, setEditingProvider] = useState<Partial<ProviderFormData> | undefined>(undefined);
    const { token } = useAuthStore();

    const fetchProviders = async () => {
        setLoading(true);
        try {
            // TODO: Refactor this into a dedicated apiService.ts file
            const response = await fetch('/api/admin/oauth/providers', {
                headers: {
                    'Authorization': `Bearer ${token}`,
                },
            });

            if (!response.ok) {
                throw new Error('Failed to fetch OAuth providers.');
            }

            const data: OAuthProvider[] = await response.json();
            setProviders(data);
        } catch (error) {
            console.error(error);
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchProviders();
    }, [token]);

    const handleAddProvider = () => {
        setEditingProvider(undefined);
        setIsModalOpen(true);
    };

    const handleEditProvider = (provider: OAuthProvider) => {
        setEditingProvider(provider);
        setIsModalOpen(true);
    };

    const handleDeleteProvider = async (providerName: string) => {
        if (!window.confirm(`Are you sure you want to delete the provider "${providerName}"?`)) {
            return;
        }

        try {
            const response = await fetch(`/api/admin/oauth/providers/${providerName}`, {
                method: 'DELETE',
                headers: {
                    'Authorization': `Bearer ${token}`,
                },
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.message || 'Failed to delete provider.');
            }

            toast.success('Provider deleted successfully!');
            fetchProviders(); // Refresh the list
        } catch (error) {
            console.error(error);
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        }
    };

    const handleSaveProvider = async (providerData: Partial<ProviderFormData>) => {
        const isEditing = !!editingProvider;
        // When editing, the provider_name cannot be changed, so we use the one from the editingProvider state.
        const providerName = isEditing ? editingProvider.provider_name : providerData.provider_name;
        
        if (!providerName) {
            toast.error("Provider name is required.");
            return;
        }

        const url = isEditing
            ? `/api/admin/oauth/providers/${providerName}`
            : '/api/admin/oauth/providers';
        const method = isEditing ? 'PUT' : 'POST';

        try {
            const response = await fetch(url, {
                method: method,
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`,
                },
                body: JSON.stringify(providerData),
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.message || 'Failed to save provider.');
            }

            toast.success(`Provider ${isEditing ? 'updated' : 'created'} successfully!`);
            setIsModalOpen(false);
            fetchProviders(); // Refresh the list
        } catch (error) {
            console.error(error);
            toast.error(error instanceof Error ? error.message : 'An unknown error occurred.');
        }
    };


    return (
        <div className="space-y-6">
            <div className="flex justify-between items-center">
                <h1 className="text-2xl font-bold">OAuth Provider Management</h1>
                <button
                    onClick={handleAddProvider}
                    className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
                >
                    Add New Provider
                </button>
            </div>

            <ProviderFormModal
                isOpen={isModalOpen}
                onClose={() => setIsModalOpen(false)}
                onSave={handleSaveProvider}
                initialData={editingProvider}
            />

            {loading ? (
                <p>Loading providers...</p>
            ) : (
                <div className="bg-white shadow overflow-hidden sm:rounded-lg">
                    <div className="overflow-x-auto">
                        <table className="min-w-full divide-y divide-gray-200">
                            <thead className="bg-gray-50">
                                <tr>
                                    <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Provider</th>
                                    <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Client ID</th>
                                    <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Enabled</th>
                                    <th scope="col" className="relative px-6 py-3">
                                        <span className="sr-only">Actions</span>
                                    </th>
                                </tr>
                            </thead>
                            <tbody className="bg-white divide-y divide-gray-200">
                                {providers.map((provider) => (
                                    <tr key={provider.provider_name}>
                                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                                            <div className="flex items-center">
                                                {provider.icon_url && (
                                                    <img
                                                        src={provider.icon_url}
                                                        alt={`${provider.provider_name} icon`}
                                                        className="w-6 h-6 mr-2"
                                                    />
                                                )}
                                                {provider.provider_name}
                                            </div>
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{provider.client_id}</td>
                                        <td className="px-6 py-4 whitespace-nowrap">
                                            <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${provider.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
                                                {provider.enabled ? 'Yes' : 'No'}
                                            </span>
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-4">
                                            <button onClick={() => handleEditProvider(provider)} className="text-indigo-600 hover:text-indigo-900">Edit</button>
                                            <button onClick={() => handleDeleteProvider(provider.provider_name)} className="text-red-600 hover:text-red-900">Delete</button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}
        </div>
    );
};

export default AdminOAuthProvidersPage;