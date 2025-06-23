// frontend/src/components/ProviderFormModal.tsx
import React, { useState, useEffect } from 'react';
import type { FormEvent } from 'react';

// This can be moved to a shared types file later
export type ProviderFormData = {
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
};

interface ProviderFormModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSave: (providerData: Partial<ProviderFormData>) => void;
    initialData?: Partial<ProviderFormData>;
}

const ProviderFormModal: React.FC<ProviderFormModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
    const [formData, setFormData] = useState<Partial<ProviderFormData>>({});
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const isEditing = !!initialData?.provider_name;

    useEffect(() => {
        if (isOpen) {
            setError(null);
            if (isEditing) {
                setFormData(initialData);
            } else {
                // Default values for a new provider
                setFormData({
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
                });
            }
        }
    }, [initialData, isOpen, isEditing]);


    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        const { name, value, type } = e.target;

        if (name.startsWith("user_info_mapping.")) {
            const mappingField = name.split('.')[1];
            setFormData(prev => ({
                ...prev,
                user_info_mapping: {
                    ...(prev.user_info_mapping || { id_field: '', username_field: '' }),
                    [mappingField]: value,
                }
            }));
        } else if (type === 'checkbox') {
            const { checked } = e.target as HTMLInputElement;
            setFormData(prev => ({ ...prev, [name]: checked }));
        } else {
            setFormData(prev => ({ ...prev, [name]: value }));
        }
    };

    const handleSubmit = async (e: FormEvent) => {
        e.preventDefault();
        setError(null);

        if (!formData.provider_name?.trim()) {
            setError("Provider Name is required.");
            return;
        }
        if (!formData.client_id?.trim()) {
            setError("Client ID is required.");
            return;
        }
        if (!isEditing && !formData.client_secret?.trim()) {
            setError("Client Secret is required for new providers.");
            return;
        }

        setIsSubmitting(true);
        try {
            await onSave(formData);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'An unknown error occurred.');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (!isOpen) {
        return null;
    }

    return (
        <div className="fixed inset-0 bg-gray-600/50 overflow-y-auto h-full w-full z-50 flex justify-center items-center">
            <div className="relative mx-auto p-8 border w-full max-w-3xl shadow-lg rounded-md bg-white">
                <form onSubmit={handleSubmit}>
                    <div className="flex justify-between items-center mb-6">
                        <h3 className="text-xl font-medium">{isEditing ? 'Edit' : 'Add New'} OAuth Provider</h3>
                        <button type="button" onClick={onClose} className="text-gray-400 hover:text-gray-600">
                            <span className="sr-only">Close</span>&times;
                        </button>
                    </div>

                    {error && <p className="text-red-500 text-sm mb-4">{error}</p>}

                    <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8">
                        {/* Left Column */}
                        <div className="space-y-4">
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Provider Name <span className="text-red-500 ml-1">*</span></label>
                                <input type="text" name="provider_name" value={formData.provider_name || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" disabled={isEditing} />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Client ID <span className="text-red-500 ml-1">*</span></label>
                                <input type="text" name="client_id" value={formData.client_id || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Client Secret {isEditing ? '' : <span className="text-red-500 ml-1">*</span>}</label>
                                <input type="password" name="client_secret" placeholder={isEditing ? 'Leave blank to keep unchanged' : ''} value={formData.client_secret || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                             <div>
                                <label className="block text-sm font-medium text-gray-700">Scopes (comma-separated)</label>
                                <input type="text" name="scopes" value={formData.scopes || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Icon URL</label>
                                <div className="flex items-center gap-2">
                                    <input type="text" name="icon_url" value={formData.icon_url || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                                    {formData.icon_url && (
                                        <img
                                            src={formData.icon_url}
                                            alt="Provider icon preview"
                                            className="w-8 h-8 object-contain"
                                            onError={(e) => {
                                                (e.target as HTMLImageElement).style.display = 'none';
                                            }}
                                        />
                                    )}
                                </div>
                            </div>
                        </div>

                        {/* Right Column */}
                        <div className="space-y-4">
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Authorization URL</label>
                                <input type="text" name="auth_url" value={formData.auth_url || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Token URL</label>
                                <input type="text" name="token_url" value={formData.token_url || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">User Info URL</label>
                                <input type="text" name="user_info_url" value={formData.user_info_url || ''} onChange={handleChange} className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                        </div>
                    </div>

                    {/* User Info Mapping Section */}
                    <div className="pt-8">
                        <h4 className="text-md font-medium text-gray-800 border-b pb-2 mb-4">User Info Field Mapping</h4>
                        <p className="text-sm text-gray-500 mt-2 mb-4">
                            Specify the field names from the provider's user info endpoint that correspond to our system's user fields.
                        </p>
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-x-6">
                            <div>
                                <label className="block text-sm font-medium text-gray-700">ID Field</label>
                                <input type="text" name="user_info_mapping.id_field" value={formData.user_info_mapping?.id_field || ''} onChange={handleChange} placeholder="e.g., id, sub" className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700">Username/Name Field</label>
                                <input type="text" name="user_info_mapping.username_field" value={formData.user_info_mapping?.username_field || ''} onChange={handleChange} placeholder="e.g., name, login" className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm" />
                            </div>
                        </div>
                    </div>

                    {/* Footer */}
                    <div className="pt-8 flex justify-between items-center">
                         <div className="flex items-center">
                            <input type="checkbox" name="enabled" checked={!!formData.enabled} onChange={handleChange} id="enabled-checkbox" className="h-4 w-4 text-indigo-600 focus:ring-indigo-500 border-gray-300 rounded" />
                            <label htmlFor="enabled-checkbox" className="ml-2 block text-sm text-gray-900">Enabled</label>
                        </div>
                        <div className="flex justify-end space-x-4">
                            <button type="button" onClick={onClose} className="px-4 py-2 bg-gray-200 text-gray-800 rounded-md hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500">
                                Cancel
                            </button>
                            <button type="submit" className="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50" disabled={isSubmitting}>
                                {isSubmitting ? 'Saving...' : 'Save'}
                            </button>
                        </div>
                    </div>
                </form>
            </div>
        </div>
    );
};

export default ProviderFormModal;