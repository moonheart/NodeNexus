import React, { useState, useEffect, useCallback } from 'react';
import { getGlobalConfig, updateGlobalConfig, retryConfigPush, pushConfig, previewConfig } from '../services/configService';
import { getAllAlertRules, createAlertRule, updateAlertRule, deleteAlertRule, updateAlertRuleStatus } from '../services/alertService';
import { getAllVpsListItems } from '../services/vpsService';
import type { AgentConfig, VpsListItemResponse, AlertRule, CreateAlertRulePayload, UpdateAlertRulePayload } from '../types';
import { useServerListStore } from '../store/serverListStore';
import AlertRuleModal from '../components/AlertRuleModal'; // Assuming this path is correct
import toast from 'react-hot-toast';
import { PlusCircle, Edit3, Trash2, ToggleLeft, ToggleRight } from 'lucide-react';

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
    const [isLoading, setIsLoading] = useState(true); // For global config
    const [error, setError] = useState<string | null>(null); // For global config
    const [isSaving, setIsSaving] = useState(false); // For global config
    const { servers } = useServerListStore(); // For VPS config status list
    const [retrying, setRetrying] = useState<number | null>(null); // For VPS config retry
    const [pushing, setPushing] = useState<number | null>(null);
    const [previewing, setPreviewing] = useState<number | null>(null);
    const [previewContent, setPreviewContent] = useState<string>('');
    const [isPreviewModalOpen, setIsPreviewModalOpen] = useState(false);


    // States for Alert Rules
    const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
    const [vpsList, setVpsList] = useState<VpsListItemResponse[]>([]);
    const [isLoadingAlerts, setIsLoadingAlerts] = useState(true);
    const [alertsError, setAlertsError] = useState<string | null>(null);
    const [isAlertModalOpen, setIsAlertModalOpen] = useState(false);
    const [currentEditingAlertRule, setCurrentEditingAlertRule] = useState<AlertRule | null>(null);

    const fetchPageData = useCallback(async () => {
        setIsLoading(true); // Combined loading state for initial page load
        setIsLoadingAlerts(true);
        try {
            const [configData, rulesData, vpsData] = await Promise.all([
                getGlobalConfig(),
                getAllAlertRules(),
                getAllVpsListItems(),
            ]);
            setConfig(configData);
            setAlertRules(rulesData);
            setVpsList(vpsData);
            setError(null);
            setAlertsError(null);
        } catch (err) {
            console.error('Failed to load settings page data:', err);
            setError('Failed to load global configuration.'); // Keep specific errors if needed
            setAlertsError('Failed to load alert rules or VPS list.');
            toast.error('Failed to load page data.');
        } finally {
            setIsLoading(false);
            setIsLoadingAlerts(false);
        }
    }, []);

    useEffect(() => {
        fetchPageData();
    }, [fetchPageData]);

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

    if (error && alertsError) { // Show combined error or separate as needed
        return <div className="container mx-auto p-4 text-red-500">Error loading page: {error} & {alertsError}</div>;
    }
    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error loading global config: {error}</div>;
    }
    if (alertsError) {
        return <div className="container mx-auto p-4 text-red-500">Error loading alert rules: {alertsError}</div>;
    }

    // Alert Rule Handlers
    const handleOpenCreateAlertModal = () => {
        setCurrentEditingAlertRule(null);
        setIsAlertModalOpen(true);
    };

    const handleOpenEditAlertModal = (rule: AlertRule) => {
        setCurrentEditingAlertRule(rule);
        setIsAlertModalOpen(true);
    };

    const handleAlertModalClose = () => {
        setIsAlertModalOpen(false);
        setCurrentEditingAlertRule(null);
    };

    const handleAlertModalSubmit = async (data: CreateAlertRulePayload | UpdateAlertRulePayload) => {
        try {
            if (currentEditingAlertRule) {
                await updateAlertRule(currentEditingAlertRule.id, data as UpdateAlertRulePayload);
                toast.success('Alert rule updated successfully!');
            } else {
                await createAlertRule(data as CreateAlertRulePayload);
                toast.success('Alert rule created successfully!');
            }
            fetchPageData(); // Refresh alert rules and potentially other data
        } catch (err) {
            console.error('Failed to save alert rule:', err);
            toast.error('Failed to save alert rule.');
            throw err;
        }
    };

    const handleDeleteAlertRule = async (id: number) => {
        if (window.confirm('Are you sure you want to delete this alert rule?')) {
            try {
                await deleteAlertRule(id);
                toast.success('Alert rule deleted successfully!');
                fetchPageData(); // Refresh alert rules
            } catch (err) {
                console.error('Failed to delete alert rule:', err);
                toast.error('Failed to delete alert rule.');
            }
        }
    };
    
    const handleToggleAlertRuleStatus = async (rule: AlertRule) => {
        try {
            const updatedRule = await updateAlertRuleStatus(rule.id, !rule.isActive);
            setAlertRules(prevRules =>
                prevRules.map(r => r.id === updatedRule.id ? updatedRule : r)
            );
            toast.success(`Rule "${updatedRule.name}" ${updatedRule.isActive ? 'enabled' : 'disabled'}.`);
        } catch (err) {
            console.error('Failed to update alert rule status:', err);
            toast.error('Failed to update alert rule status.');
        }
    };
    
    
    return (
        <div className="container mx-auto p-4 space-y-8">
            <h1 className="text-3xl font-bold mb-6">Settings</h1>

            {/* Global Agent Configuration Section */}
            <section className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">Global Agent Configuration</h2>
                {isLoading && !config && <p>Loading global config...</p>}
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

            {/* Alert Rules Section */}
            <section className="bg-white p-6 rounded-lg shadow-md">
                <div className="flex justify-between items-center mb-4">
                    <h2 className="text-xl font-semibold">Alert Rules</h2>
                    <button
                        onClick={handleOpenCreateAlertModal}
                        className="bg-blue-500 hover:bg-blue-600 text-white font-semibold py-2 px-4 rounded-lg shadow-sm flex items-center"
                    >
                        <PlusCircle size={18} className="mr-2" /> Add New Rule
                    </button>
                </div>
                {isLoadingAlerts && <p>Loading alert rules...</p>}
                {!isLoadingAlerts && alertRules.length === 0 && <p className="text-gray-500">No alert rules configured yet.</p>}
                {!isLoadingAlerts && alertRules.length > 0 && (
                    <ul className="divide-y divide-gray-200">
                        {alertRules.map(rule => (
                            <li key={rule.id} className="py-3 flex justify-between items-center">
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center">
                                        <p className="text-md font-medium text-gray-900 mr-2">{rule.name}</p>
                                        <span className={`px-2 py-0.5 inline-flex text-xs leading-4 font-semibold rounded-full ${rule.isActive ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
                                            {rule.isActive ? 'Active' : 'Inactive'}
                                        </span>
                                    </div>
                                    <p className="text-sm text-gray-500">
                                        Metric: {rule.metricType}, Threshold: {rule.comparisonOperator} {rule.threshold} for {rule.durationSeconds}s
                                    </p>
                                    <p className="text-sm text-gray-500">
                                        Channels: {rule.notificationChannelIds?.join(', ') || 'None'}
                                    </p>
                                    <p className="text-sm text-gray-500">
                                        Cooldown: {rule.cooldownSeconds !== undefined ? `${rule.cooldownSeconds}s` : 'Default (300s)'}
                                    </p>
                                </div>
                                <div className="flex items-center space-x-2">
                                    <button
                                        onClick={() => handleToggleAlertRuleStatus(rule)}
                                        className={`p-1 rounded-md ${rule.isActive ? 'text-gray-500 hover:text-gray-700' : 'text-gray-400 hover:text-gray-600'}`}
                                        title={rule.isActive ? 'Disable Rule' : 'Enable Rule'}
                                    >
                                        {rule.isActive ? <ToggleRight size={20} className="text-green-600" /> : <ToggleLeft size={20} className="text-red-600"/>}
                                    </button>
                                    <button onClick={() => handleOpenEditAlertModal(rule)} className="text-indigo-600 hover:text-indigo-900 p-1"><Edit3 size={18} /></button>
                                    <button onClick={() => handleDeleteAlertRule(rule.id)} className="text-red-600 hover:text-red-900 p-1"><Trash2 size={18} /></button>
                                </div>
                            </li>
                        ))}
                    </ul>
                )}
            </section>
            
            {isAlertModalOpen && (
                <AlertRuleModal
                    isOpen={isAlertModalOpen}
                    onClose={handleAlertModalClose}
                    onRuleSaved={handleAlertModalSubmit}
                    rule={currentEditingAlertRule}
                    vpsList={vpsList}
                />
            )}

            {/* VPS Configuration Status Section */}
            <section className="bg-white p-6 rounded-lg shadow-md">
                <h2 className="text-xl font-semibold mb-4">VPS Configuration Status</h2>
                {isLoading && servers.length === 0 && <p>Loading VPS statuses...</p>}
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

export default SettingsPage;