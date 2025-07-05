import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { getAllAlertRules, deleteAlertRule, updateAlertRuleStatus } from '../services/alertService';
import { getAllVpsListItems } from '../services/vpsService';
import type { VpsListItemResponse, AlertRule } from '../types';
import AlertRuleModal from '../components/AlertRuleModal';
import toast from 'react-hot-toast';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardAction } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Skeleton } from '@/components/ui/skeleton';
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
import EmptyState from '@/components/EmptyState';

const AlertsTableSkeleton: React.FC = () => {
    const { t } = useTranslation();
    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>{t('alertSettings.table.name')}</TableHead>
                    <TableHead>{t('alertSettings.table.status')}</TableHead>
                    <TableHead>{t('alertSettings.table.condition')}</TableHead>
                    <TableHead className="text-center">{t('alertSettings.table.enabled')}</TableHead>
                    <TableHead className="text-right">{t('alertSettings.table.actions')}</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {[...Array(3)].map((_, i) => (
                    <TableRow key={i}>
                        <TableCell><Skeleton className="h-5 w-32" /></TableCell>
                        <TableCell><Skeleton className="h-6 w-20" /></TableCell>
                        <TableCell>
                            <Skeleton className="h-4 w-full" />
                            <Skeleton className="h-3 w-3/4 mt-2" />
                        </TableCell>
                        <TableCell className="text-center"><Skeleton className="h-6 w-12 mx-auto" /></TableCell>
                        <TableCell className="text-right space-x-1">
                            <Skeleton className="h-8 w-8 inline-block" />
                            <Skeleton className="h-8 w-8 inline-block" />
                        </TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
};


const AlertsSettingsPage: React.FC = () => {
    const { t } = useTranslation();
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
            const errorMessage = err instanceof Error ? err.message : t('alertSettings.notifications.loadFailed');
            console.error(t('alertSettings.notifications.loadFailed'), err);
            setError(errorMessage);
            toast.error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [t]);

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
        toast.success(t(currentEditingAlertRule ? 'alertSettings.notifications.updateSuccess' : 'alertSettings.notifications.createSuccess'));
        fetchAlertsData();
    };

    const handleDeleteClick = (id: number) => {
        setDeletingRuleId(id);
        setIsDeleteAlertOpen(true);
    };

    const confirmDeleteAlertRule = async () => {
        if (deletingRuleId === null) return;
        const toastId = toast.loading(t('alertSettings.notifications.deleting'));
        try {
            await deleteAlertRule(deletingRuleId);
            toast.success(t('alertSettings.notifications.deleteSuccess'), { id: toastId });
            fetchAlertsData();
        } catch (err) {
            console.error(t('alertSettings.notifications.deleteError'), err);
            toast.error(t('alertSettings.notifications.deleteError'), { id: toastId });
        } finally {
            setIsDeleteAlertOpen(false);
            setDeletingRuleId(null);
        }
    };

    const handleToggleAlertRuleStatus = async (rule: AlertRule) => {
        const toastId = toast.loading(t('alertSettings.notifications.updatingStatus', { ruleName: rule.name }));
        try {
            const updatedRule = await updateAlertRuleStatus(rule.id, !rule.isActive);
            setAlertRules(prevRules =>
                prevRules.map(r => r.id === updatedRule.id ? updatedRule : r)
            );
            toast.success(t(updatedRule.isActive ? 'alertSettings.notifications.enabledSuccess' : 'alertSettings.notifications.disabledSuccess', { ruleName: updatedRule.name }), { id: toastId });
        } catch (err) {
            console.error(t('alertSettings.notifications.updateStatusError'), err);
            toast.error(t('alertSettings.notifications.updateStatusError'), { id: toastId });
        }
    };

    if (error) {
        return <div className="container mx-auto p-4 text-destructive">{t('alertSettings.errorLoading', { error: error })}</div>;
    }

    return (
        <div className="space-y-6">
            <Card>
                <CardHeader >
                    <CardTitle>{t('alertSettings.title')}</CardTitle>
                    <CardDescription>{t('alertSettings.description')}</CardDescription>
                    <CardAction>
                        <Button onClick={handleOpenCreateAlertModal}>
                            <PlusCircle size={18} className="mr-2" /> {t('alertSettings.addNewRule')}
                        </Button>
                    </CardAction>
                </CardHeader>
                <CardContent>
                    {isLoading ? (
                        <AlertsTableSkeleton />
                    ) : alertRules.length === 0 ? (
                        <EmptyState
                            title={t('alertSettings.empty.title')}
                            message={t('alertSettings.empty.message')}
                            action={<Button onClick={handleOpenCreateAlertModal}><PlusCircle size={18} className="mr-2" /> {t('alertSettings.addNewRule')}</Button>}
                        />
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>{t('alertSettings.table.name')}</TableHead>
                                    <TableHead>{t('alertSettings.table.status')}</TableHead>
                                    <TableHead>{t('alertSettings.table.condition')}</TableHead>
                                    <TableHead className="text-center">{t('alertSettings.table.enabled')}</TableHead>
                                    <TableHead className="text-right">{t('alertSettings.table.actions')}</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {alertRules.map(rule => (
                                    <TableRow key={rule.id}>
                                        <TableCell className="font-medium">{rule.name}</TableCell>
                                        <TableCell>
                                            <Badge variant={rule.isActive ? 'success' : 'secondary'}>
                                                {rule.isActive ? t('alertSettings.status.active') : t('alertSettings.status.inactive')}
                                            </Badge>
                                        </TableCell>
                                        <TableCell>
                                            <div className="text-sm">
                                                <span className="font-semibold">{t(`alertSettings.metrics.${rule.metricType}` as const)}</span>
                                                <span> {rule.comparisonOperator} </span>
                                                <span className="font-semibold">{rule.threshold}</span>
                                                <span> {t('alertSettings.condition.for')} </span>
                                                <span className="font-semibold">{t('alertSettings.condition.seconds', { count: rule.durationSeconds })}</span>
                                            </div>
                                            <div className="text-xs text-muted-foreground">
                                                {t('alertSettings.condition.cooldown', { count: rule.cooldownSeconds })}
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
                        <AlertDialogTitle>{t('common.dialogs.delete.title')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('alertSettings.deleteDialog.description')}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel onClick={() => setDeletingRuleId(null)}>{t('common.actions.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmDeleteAlertRule}>{t('common.actions.delete')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};

export default AlertsSettingsPage;