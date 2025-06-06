import React, { useState, useEffect } from 'react';
import { getGlobalConfig, updateGlobalConfig, retryConfigPush } from '../services/configService';
import type { AgentConfig } from '../types';
import { useServerListStore } from '../store/serverListStore';
import type { VpsListItemResponse } from '../types';

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


const SettingsPage: React.FC = () => {
    const [config, setConfig] = useState<AgentConfig | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isSaving, setIsSaving] = useState(false);
    const { servers } = useServerListStore();
    const [retrying, setRetrying] = useState<number | null>(null);


    useEffect(() => {
        const fetchConfig = async () => {
            try {
                setIsLoading(true);
                const data = await getGlobalConfig();
                setConfig(data);
                setError(null);
            } catch (err) {
                setError('Failed to load global configuration.');
                console.error(err);
            } finally {
                setIsLoading(false);
            }
        };

        fetchConfig();
    }, []);

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
            alert('Configuration saved successfully! It will be pushed to relevant agents.');
        } catch (err) {
            setError('Failed to save configuration.');
            console.error(err);
            alert('Error: Failed to save configuration.');
        } finally {
            setIsSaving(false);
        }
    };

    const handleRetry = async (vpsId: number) => {
        setRetrying(vpsId);
        try {
            await retryConfigPush(vpsId);
            alert(`Retrying config push for VPS ID: ${vpsId}`);
            // Note: The status will update via the WebSocket connection eventually.
        } catch (err) {
            console.error(`Failed to retry config push for VPS ID: ${vpsId}`, err);
            alert(`Error: Failed to retry config push for VPS ID: ${vpsId}`);
        } finally {
            setRetrying(null);
        }
    };

    if (isLoading) {
        return <div className="container mx-auto p-4">Loading configuration...</div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error: {error}</div>;
    }

    return (
        <div className="container mx-auto p-4">
            <h1 className="text-2xl font-bold mb-4">Settings</h1>

            <form onSubmit={handleSave} className="bg-white p-6 rounded-lg shadow-md mb-8">
                <h2 className="text-xl font-semibold mb-4">Global Agent Configuration</h2>
                
                {config && (
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                        {Object.keys(config).filter(k => k !== 'feature_flags').map((key) => (
                            <div key={key}>
                                <label htmlFor={key} className="block text-sm font-medium text-gray-700 capitalize">
                                    {key.replace(/_/g, ' ')}
                                </label>
                                <input
                                    type={typeof config[key as keyof AgentConfig] === 'number' ? 'number' : 'text'}
                                    id={key}
                                    name={key}
                                    value={String(config[key as keyof AgentConfig])}
                                    onChange={handleInputChange}
                                    className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                                />
                            </div>
                        ))}
                    </div>
                )}

                <div className="mt-6">
                    <button
                        type="submit"
                        disabled={isSaving}
                        className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-300"
                    >
                        {isSaving ? 'Saving...' : 'Save Global Config'}
                    </button>
                </div>
            </form>

            <div className="bg-white p-6 rounded-lg shadow-md">
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
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
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
            </div>
        </div>
    );
};

export default SettingsPage;