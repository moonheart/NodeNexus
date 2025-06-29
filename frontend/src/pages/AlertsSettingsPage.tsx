import React, { useState, useEffect, useCallback } from 'react';
import { getAllAlertRules, deleteAlertRule, updateAlertRuleStatus } from '../services/alertService';
import { getAllVpsListItems } from '../services/vpsService';
import type { VpsListItemResponse, AlertRule } from '../types';
import AlertRuleModal from '../components/AlertRuleModal';
import toast from 'react-hot-toast';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
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
import { Edit3, PlusCircle, Trash2 } from 'lucide-react';
import { RefreshCwIcon as SpinnerIcon } from '@/components/Icons';
import EmptyState from '@/components/EmptyState';

const AlertsSettingsPage: React.FC = () => {
    const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
    const [vpsList, setVpsList] = useState<VpsListItemResponse[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [isAlertModalOpen, setIsAlertModalOpen] = useState(false);
    const [currentEditingAlertRule, setCurrentEditingAlertRule] = useState<AlertRule | null>(null);
    const [isDeleteAlertOpen, setIsDeleteAlertOpen] = useState(false);
    const [deletingRuleId, setDeletingRuleId] = useState<number | null>(null);

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
            const errorMessage = err instanceof Error ? err.message : 'Failed to load alert rules or VPS list.';
            console.error(errorMessage, err);
            setError(errorMessage);
            toast.error(errorMessage);
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

    const handleRuleSaved = () => {
        toast.success(`Alert rule ${currentEditingAlertRule ? 'updated' : 'created'} successfully!`);
        fetchAlertsData();
    };

    const handleDeleteClick = (id: number) => {
        setDeletingRuleId(id);
        setIsDeleteAlertOpen(true);
    };

    const confirmDeleteAlertRule = async () => {
        if (deletingRuleId === null) return;
        const toastId = toast.loading('Deleting alert rule...');
        try {
            await deleteAlertRule(deletingRuleId);
            toast.success('Alert rule deleted successfully!', { id: toastId });
            fetchAlertsData();
        } catch (err) {
            console.error('Failed to delete alert rule:', err);
            toast.error('Failed to delete alert rule.', { id: toastId });
        } finally {
            setIsDeleteAlertOpen(false);
            setDeletingRuleId(null);
        }
    };

    const handleToggleAlertRuleStatus = async (rule: AlertRule) => {
        const toastId = toast.loading(`Updating status for "${rule.name}"...`);
        try {
            const updatedRule = await updateAlertRuleStatus(rule.id, !rule.isActive);
            setAlertRules(prevRules =>
                prevRules.map(r => r.id === updatedRule.id ? updatedRule : r)
            );
            toast.success(`Rule "${updatedRule.name}" ${updatedRule.isActive ? 'enabled' : 'disabled'}.`, { id: toastId });
        } catch (err) {
            console.error('Failed to update alert rule status:', err);
            toast.error('Failed to update alert rule status.', { id: toastId });
        }
    };

    if (isLoading) {
        return <div className="flex items-center justify-center h-full"><SpinnerIcon className="h-8 w-8 animate-spin" /></div>;
    }

    if (error) {
        return <div className="container mx-auto p-4 text-red-500">Error loading alert rules: {error}</div>;
    }

    return (
        <div className="space-y-6">
            <Card>
                <CardHeader className="flex flex-row items-center justify-between">
                    <div>
                        <CardTitle>Alert Rules</CardTitle>
                        <CardDescription>Manage rules to get notified about important events.</CardDescription>
                    </div>
                    <Button onClick={handleOpenCreateAlertModal}>
                        <PlusCircle size={18} className="mr-2" /> Add New Rule
                    </Button>
                </CardHeader>
                <CardContent>
                    {alertRules.length === 0 ? (
                        <EmptyState
                            title="No Alert Rules"
                            message="Get started by creating a new alert rule."
                            action={<Button onClick={handleOpenCreateAlertModal}><PlusCircle size={18} className="mr-2" /> Add New Rule</Button>}
                        />
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Name</TableHead>
                                    <TableHead>Status</TableHead>
                                    <TableHead>Condition</TableHead>
                                    <TableHead className="text-center">Enabled</TableHead>
                                    <TableHead className="text-right">Actions</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {alertRules.map(rule => (
                                    <TableRow key={rule.id}>
                                        <TableCell className="font-medium">{rule.name}</TableCell>
                                        <TableCell>
                                            <Badge variant={rule.isActive ? 'success' : 'secondary'}>
                                                {rule.isActive ? 'Active' : 'Inactive'}
                                            </Badge>
                                        </TableCell>
                                        <TableCell>
                                            <div className="text-sm">
                                                <span className="font-semibold">{rule.metricType.replace(/_/g, ' ')}</span>
                                                <span> {rule.comparisonOperator} </span>
                                                <span className="font-semibold">{rule.threshold}</span>
                                                <span> for </span>
                                                <span className="font-semibold">{rule.durationSeconds}s</span>
                                            </div>
                                            <div className="text-xs text-muted-foreground">
                                                Cooldown: {rule.cooldownSeconds}s
                                            </div>
                                        </TableCell>
                                        <TableCell className="text-center">
                                            <Switch
                                                checked={rule.isActive}
                                                onCheckedChange={() => handleToggleAlertRuleStatus(rule)}
                                                aria-label={`Toggle rule ${rule.name}`}
                                            />
                                        </TableCell>
                                        <TableCell className="text-right space-x-1">
                                            <Button variant="ghost" size="icon" onClick={() => handleOpenEditAlertModal(rule)}>
                                                <Edit3 className="h-4 w-4" />
                                            </Button>
                                            <Button variant="ghost" size="icon" onClick={() => handleDeleteClick(rule.id)}>
                                                <Trash2 className="h-4 w-4 text-destructive" />
                                            </Button>
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    )}
                </CardContent>
            </Card>

            <AlertRuleModal
                isOpen={isAlertModalOpen}
                onOpenChange={setIsAlertModalOpen}
                onRuleSaved={handleRuleSaved}
                rule={currentEditingAlertRule}
                vpsList={vpsList}
            />

            <AlertDialog open={isDeleteAlertOpen} onOpenChange={setIsDeleteAlertOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Are you sure?</AlertDialogTitle>
                        <AlertDialogDescription>
                            This action cannot be undone. This will permanently delete the alert rule.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setDeletingRuleId(null)}>Cancel</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmDeleteAlertRule}>Delete</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default AlertsSettingsPage;