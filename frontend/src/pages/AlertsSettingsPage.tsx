import React, { useState, useEffect, useCallback } from 'react';
import { getAllAlertRules, createAlertRule, updateAlertRule, deleteAlertRule, updateAlertRuleStatus } from '../services/alertService';
import { getAllVpsListItems } from '../services/vpsService';
import type { VpsListItemResponse, AlertRule, CreateAlertRulePayload, UpdateAlertRulePayload } from '../types';
import AlertRuleModal from '../components/AlertRuleModal';
import toast from 'react-hot-toast';
import { PlusCircle, Edit3, Trash2, ToggleLeft, ToggleRight } from 'lucide-react';

const AlertsSettingsPage: React.FC = () => {
    const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
    const [vpsList, setVpsList] = useState<VpsListItemResponse[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isAlertModalOpen, setIsAlertModalOpen] = useState(false);
    const [currentEditingAlertRule, setCurrentEditingAlertRule] = useState<AlertRule | null>(null);

    const fetchAlertsData = useCallback(async () => {
        setIsLoading(true);
        try {
            const [rulesData, vpsData] = await Promise.all([
                getAllAlertRules(),
                getAllVpsListItems(),
            ]);
            setAlertRules(rulesData);
            setVpsList(vpsData);
            setError(null);
        } catch (err) {
            console.error('Failed to load alert rules or VPS list:', err);
            setError('Failed to load alert rules or VPS list.');
            toast.error('Failed to load alert rules or VPS list.');
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        fetchAlertsData();
    }, [fetchAlertsData]);

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
            fetchAlertsData();
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
                fetchAlertsData();
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

    if (isLoading) {
        return <div className="container mx-auto p-4">Loading alert rules...</div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error loading alert rules: {error}</div>;
    }

    return (
        <div className="space-y-8">
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
                {alertRules.length === 0 && <p className="text-gray-500">No alert rules configured yet.</p>}
                {alertRules.length > 0 && (
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
        </div>
    );
};

export default AlertsSettingsPage;