import React, { useState, useEffect, useCallback } from 'react';
import { getGlobalConfig, updateGlobalConfig, retryConfigPush, pushConfig, previewConfig } from '../services/configService';
import type { AgentConfig, VpsListItemResponse } from '../types';
import { useServerListStore } from '../store/serverListStore';
import toast from 'react-hot-toast';

const ConfigStatusBadge: React.FC<{ status: string }> = ({ status }) => {
    const statusMap: { [key: string]: { text: string; className: string } } = {
        synced: { text: 'Synced', className: 'bg-green-100 text-green-800' },
        pending: { text: 'Pending', className: 'bg-yellow-100 text-yellow-800' },
        failed: { text: 'Failed', className: 'bg-red-100 text-red-800' },
        unknown: { text: 'Unknown', className: 'bg-gray-100 text-gray-800' },
    };
    const { text, className } = statusMap[status] || statusMap.unknown;
    return (
        <span className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${className}`}>
            {text}
        </span>
    );
};

const GlobalSettingsPage: React.FC = () => {
    const [config, setConfig] = useState<AgentConfig | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isSaving, setIsSaving] = useState(false);
    const { servers } = useServerListStore();
    const [retrying, setRetrying] = useState<number | null>(null);
    const [pushing, setPushing] = useState<number | null>(null);
    const [previewing, setPreviewing] = useState<number | null>(null);
    const [previewContent, setPreviewContent] = useState<string>('');
    const [isPreviewModalOpen, setIsPreviewModalOpen] = useState(false);

    const fetchConfig = useCallback(async () => {
        setIsLoading(true);
        try {
            const configData = await getGlobalConfig();
            setConfig(configData);
            setError(null);
        } catch (err) {
            console.error('Failed to load global configuration:', err);
            setError('Failed to load global configuration.');
            toast.error('Failed to load global configuration.');
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        fetchConfig();
    }, [fetchConfig]);

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!config) return;
        const { name, value, type } = e.target;
        setConfig({
            ...config,
            [name]: type === 'number' ? Number(value) : value,
        });
    };

    const handleSave = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!config) return;

        setIsSaving(true);
        setError(null);
        try {
            await updateGlobalConfig(config);
            toast.success('Configuration saved successfully! It will be pushed to relevant agents.');
        } catch (err) {
            setError('Failed to save configuration.');
            console.error(err);
            toast.error('Error: Failed to save configuration.');
        } finally {
            setIsSaving(false);
        }
    };

    const handleRetry = async (vpsId: number) => {
        setRetrying(vpsId);
        try {
            await retryConfigPush(vpsId);
            toast.success(`Retrying config push for VPS ID: ${vpsId}`);
        } catch (err) {
            console.error(`Failed to retry config push for VPS ID: ${vpsId}`, err);
            toast.error(`Error: Failed to retry config push for VPS ID: ${vpsId}`);
        } finally {
            setRetrying(null);
        }
    };

    const handlePushConfig = async (vpsId: number) => {
        setPushing(vpsId);
        try {
            await pushConfig(vpsId);
            toast.success(`Configuration push triggered for VPS ID: ${vpsId}`);
        } catch (err) {
            console.error(`Failed to trigger config push for VPS ID: ${vpsId}`, err);
            toast.error(`Error: Failed to trigger config push for VPS ID: ${vpsId}`);
        } finally {
            setPushing(null);
        }
    };

    const handlePreviewConfig = async (vpsId: number) => {
        setPreviewing(vpsId);
        try {
            const config = await previewConfig(vpsId);
            setPreviewContent(JSON.stringify(config, null, 2));
            setIsPreviewModalOpen(true);
        } catch (err) {
            console.error(`Failed to preview config for VPS ID: ${vpsId}`, err);
            toast.error(`Error: Failed to preview config for VPS ID: ${vpsId}`);
        } finally {
            setPreviewing(null);
        }
    };

    if (isLoading) {
        return <div className="container mx-auto p-4">Loading configuration...</div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error loading global config: {error}</div>;
    }

    return (
        <div className="space-y-8">
            <section className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">Global Agent Configuration</h2>
                {config && (
                    <form onSubmit={handleSave}>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                            {Object.keys(config).filter(k => k !== 'feature_flags').map((key) => (
                                <div key={key}>
                                    <label htmlFor={`global-${key}`} className="block text-sm font-medium text-gray-700 capitalize">
                                        {key.replace(/_/g, ' ')}
                                    </label>
                                    <input
                                        type={typeof config[key as keyof AgentConfig] === 'number' ? 'number' : 'text'}
                                        id={`global-${key}`}
                                        name={key}
                                        value={String(config[key as keyof AgentConfig])}
                                        onChange={handleInputChange}
                                        className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                                    />
                                </div>
                            ))}
                        </div>
                        <div className="mt-6">
                            <button
                                type="submit"
                                disabled={isSaving}
                                className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50"
                            >
                                {isSaving ? 'Saving...' : 'Save Global Config'}
                            </button>
                        </div>
                    </form>
                )}
            </section>

            <section className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">VPS Configuration Status</h2>
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200">
                        <thead className="bg-gray-50">
                            <tr>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Name</th>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Config Status</th>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Last Update</th>
                                <th scope="col" className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Last Error</th>
                                <th scope="col" className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                            </tr>
                        </thead>
                        <tbody className="bg-white divide-y divide-gray-200">
                            {servers.map((vps: VpsListItemResponse) => (
                                <tr key={vps.id}>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">{vps.name}</td>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500"><ConfigStatusBadge status={vps.configStatus} /></td>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{vps.lastConfigUpdateAt ? new Date(vps.lastConfigUpdateAt).toLocaleString() : 'N/A'}</td>
                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-red-600">{vps.lastConfigError || 'None'}</td>
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-2">
                                        <button
                                            onClick={() => handlePreviewConfig(vps.id)}
                                            disabled={previewing === vps.id}
                                            className="text-blue-600 hover:text-blue-900 disabled:text-gray-400"
                                        >
                                            {previewing === vps.id ? 'Loading...' : 'Preview'}
                                        </button>
                                        <button
                                            onClick={() => handlePushConfig(vps.id)}
                                            disabled={pushing === vps.id}
                                            className="text-green-600 hover:text-green-900 disabled:text-gray-400"
                                        >
                                            {pushing === vps.id ? 'Pushing...' : 'Push Config'}
                                        </button>
                                        {vps.configStatus === 'failed' && (
                                            <button
                                                onClick={() => handleRetry(vps.id)}
                                                disabled={retrying === vps.id}
                                                className="text-indigo-600 hover:text-indigo-900 disabled:text-gray-400"
                                            >
                                                {retrying === vps.id ? 'Retrying...' : 'Retry'}
                                            </button>
                                        )}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </section>

            {isPreviewModalOpen && (
                <div className="fixed inset-0 bg-gray-600/50 overflow-y-auto h-full w-full z-50">
                    <div className="relative top-20 mx-auto p-5 border w-1/2 shadow-lg rounded-md bg-white">
                        <div className="mt-3 text-center">
                            <h3 className="text-lg leading-6 font-medium text-gray-900">Configuration Preview</h3>
                            <div className="mt-2 px-7 py-3">
                                <pre className="bg-gray-100 p-4 rounded-md text-left text-sm overflow-auto max-h-96">
                                    <code>{previewContent}</code>
                                </pre>
                            </div>
                            <div className="items-center px-4 py-3">
                                <button
                                    id="ok-btn"
                                    onClick={() => setIsPreviewModalOpen(false)}
                                    className="px-4 py-2 bg-gray-800 text-white text-base font-medium rounded-md w-full shadow-sm hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500"
                                >
                                    Close
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};

export default GlobalSettingsPage;