import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { getGlobalConfig, updateGlobalConfig, retryConfigPush, pushConfig, previewConfig } from '../services/configService';
import type { AgentConfig, VpsListItemResponse } from '../types';
import { useServerListStore } from '../store/serverListStore';
import toast from 'react-hot-toast';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { RefreshCwIcon } from '@/components/Icons';
import { Skeleton } from '@/components/ui/skeleton';

const ConfigStatusBadge: React.FC<{ status: string }> = ({ status }) => {
    const { t } = useTranslation();
    const statusMap: { [key: string]: { text: string; variant: "default" | "destructive" | "outline" | "secondary" | "success" | "warning" } } = {
        synced: { text: t('agentSettings.configStatus.synced'), variant: 'success' },
        pending: { text: t('agentSettings.configStatus.pending'), variant: 'warning' },
        failed: { text: t('agentSettings.configStatus.failed'), variant: 'destructive' },
        unknown: { text: t('agentSettings.configStatus.unknown'), variant: 'secondary' },
    };
    const { text, variant } = statusMap[status] || statusMap.unknown;
    return (
        <Badge variant={variant}>{text}</Badge>
    );
};

const GlobalSettingsPage: React.FC = () => {
    const { t } = useTranslation();
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
            const errorMessage = err instanceof Error ? err.message : t('agentSettings.notifications.loadConfigFailed');
            setError(errorMessage);
            toast.error(errorMessage);
        } finally {
            setIsLoading(false);
        }
    }, [t]);

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
        const toastId = toast.loading(t('agentSettings.notifications.saving'));
        try {
            await updateGlobalConfig(config);
            toast.success(t('agentSettings.notifications.saveSuccess'), { id: toastId });
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : t('agentSettings.notifications.saveFailed');
            setError(errorMessage);
            console.error(err);
            toast.error(t('common.notifications.error', { error: errorMessage }), { id: toastId });
        } finally {
            setIsSaving(false);
        }
    };

    const handleRetry = async (vpsId: number) => {
        setRetrying(vpsId);
        const toastId = toast.loading(t('agentSettings.notifications.retrying', { vpsId }));
        try {
            await retryConfigPush(vpsId);
            toast.success(t('agentSettings.notifications.retryInitiated', { vpsId }), { id: toastId });
        } catch (err) {
            console.error(t('agentSettings.notifications.retryFailed', { vpsId }), err);
            toast.error(t('common.notifications.error', { error: err instanceof Error ? err.message : t('common.errors.unknown') }), { id: toastId });
        } finally {
            setRetrying(null);
        }
    };

    const handlePushConfig = async (vpsId: number) => {
        setPushing(vpsId);
        const toastId = toast.loading(t('agentSettings.notifications.pushing', { vpsId }));
        try {
            await pushConfig(vpsId);
            toast.success(t('agentSettings.notifications.pushTriggered', { vpsId }), { id: toastId });
        } catch (err) {
            console.error(t('agentSettings.notifications.pushFailed', { vpsId }), err);
            toast.error(t('common.notifications.error', { error: err instanceof Error ? err.message : t('common.errors.unknown') }), { id: toastId });
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
            console.error(t('agentSettings.notifications.previewFailed', { vpsId }), err);
            toast.error(t('common.notifications.error', { error: err instanceof Error ? err.message : t('agentSettings.notifications.previewFailed', { vpsId }) }));
        } finally {
            setPreviewing(null);
        }
    };

    if (isLoading) {
        return (
            <div className="space-y-6">
                <Card>
                    <CardHeader>
                        <Skeleton className="h-7 w-1/4 mb-2" />
                        <Skeleton className="h-4 w-1/2" />
                    </CardHeader>
                    <CardContent>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                            {Array.from({ length: 9 }).map((_, index) => (
                                <div className="space-y-2" key={index}>
                                    <Skeleton className="h-4 w-1/3" />
                                    <Skeleton className="h-10 w-full" />
                                </div>
                            ))}
                        </div>
                        <div className="mt-6 flex justify-end">
                            <Skeleton className="h-10 w-24" />
                        </div>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <Skeleton className="h-7 w-1/3 mb-2" />
                        <Skeleton className="h-4 w-2/3" />
                    </CardHeader>
                    <CardContent>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead><Skeleton className="h-5 w-20" /></TableHead>
                                    <TableHead><Skeleton className="h-5 w-24" /></TableHead>
                                    <TableHead><Skeleton className="h-5 w-32" /></TableHead>
                                    <TableHead><Skeleton className="h-5 w-40" /></TableHead>
                                    <TableHead className="text-right"><Skeleton className="h-5 w-28 ml-auto" /></TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {Array.from({ length: 3 }).map((_, index) => (
                                    <TableRow key={index}>
                                        <TableCell><Skeleton className="h-5 w-24" /></TableCell>
                                        <TableCell><Skeleton className="h-6 w-20" /></TableCell>
                                        <TableCell><Skeleton className="h-5 w-40" /></TableCell>
                                        <TableCell><Skeleton className="h-5 w-28" /></TableCell>
                                        <TableCell className="text-right space-x-2">
                                            <Skeleton className="h-8 w-16 inline-block" />
                                            <Skeleton className="h-8 w-16 inline-block" />
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </CardContent>
                </Card>
            </div>
        );
    }

    if (error) {
        return (
            <Alert variant="destructive">
                <AlertTitle>{t('common.errors.title')}</AlertTitle>
                <AlertDescription>{error}</AlertDescription>
            </Alert>
        );
    }

    return (
        <div className="space-y-6">
            <Dialog open={isPreviewModalOpen} onOpenChange={setIsPreviewModalOpen}>
                <DialogContent className="sm:max-w-[600px]">
                    <DialogHeader>
                        <DialogTitle>{t('agentSettings.previewModal.title')}</DialogTitle>
                    </DialogHeader>
                    <div className="mt-2">
                        <pre className="bg-muted p-4 rounded-md text-sm overflow-auto max-h-[60vh]">
                            <code>{previewContent}</code>
                        </pre>
                    </div>
                    <DialogFooter>
                        <Button onClick={() => setIsPreviewModalOpen(false)}>{t('common.actions.close')}</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>

            <Card>
                <CardHeader>
                    <CardTitle>{t('agentSettings.title')}</CardTitle>
                    <CardDescription>{t('agentSettings.description')}</CardDescription>
                </CardHeader>
                <CardContent>
                    {config && (
                        <form onSubmit={handleSave}>
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div className="space-y-2">
                                    <Label htmlFor="metricsCollectIntervalSeconds">{t('agentSettings.labels.metricsCollectInterval')}</Label>
                                    <Input id="metricsCollectIntervalSeconds" name="metricsCollectIntervalSeconds" type="number" value={config.metricsCollectIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="metricsUploadBatchMaxSize">{t('agentSettings.labels.metricsUploadBatchSize')}</Label>
                                    <Input id="metricsUploadBatchMaxSize" name="metricsUploadBatchMaxSize" type="number" value={config.metricsUploadBatchMaxSize} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="metricsUploadIntervalSeconds">{t('agentSettings.labels.metricsUploadInterval')}</Label>
                                    <Input id="metricsUploadIntervalSeconds" name="metricsUploadIntervalSeconds" type="number" value={config.metricsUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="dockerInfoCollectIntervalSeconds">{t('agentSettings.labels.dockerInfoCollectInterval')}</Label>
                                    <Input id="dockerInfoCollectIntervalSeconds" name="dockerInfoCollectIntervalSeconds" type="number" value={config.dockerInfoCollectIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="dockerInfoUploadIntervalSeconds">{t('agentSettings.labels.dockerInfoUploadInterval')}</Label>
                                    <Input id="dockerInfoUploadIntervalSeconds" name="dockerInfoUploadIntervalSeconds" type="number" value={config.dockerInfoUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="genericMetricsUploadBatchMaxSize">{t('agentSettings.labels.genericMetricsBatchSize')}</Label>
                                    <Input id="genericMetricsUploadBatchMaxSize" name="genericMetricsUploadBatchMaxSize" type="number" value={config.genericMetricsUploadBatchMaxSize} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="genericMetricsUploadIntervalSeconds">{t('agentSettings.labels.genericMetricsUploadInterval')}</Label>
                                    <Input id="genericMetricsUploadIntervalSeconds" name="genericMetricsUploadIntervalSeconds" type="number" value={config.genericMetricsUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="logLevel">{t('agentSettings.labels.logLevel')}</Label>
                                    <Input id="logLevel" name="logLevel" type="text" value={config.logLevel} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="heartbeatIntervalSeconds">{t('agentSettings.labels.heartbeatInterval')}</Label>
                                    <Input id="heartbeatIntervalSeconds" name="heartbeatIntervalSeconds" type="number" value={config.heartbeatIntervalSeconds} onChange={handleInputChange} />
                                </div>
                            </div>
                            <div className="mt-6 flex justify-end">
                                <Button type="submit" disabled={isSaving}>
                                    {isSaving && <RefreshCwIcon className="mr-2 h-4 w-4 animate-spin" />}
                                    {isSaving ? t('common.status.saving') : t('agentSettings.actions.save')}
                                </Button>
                            </div>
                        </form>
                    )}
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>{t('agentSettings.vpsStatusTitle')}</CardTitle>
                    <CardDescription>{t('agentSettings.vpsStatusDescription')}</CardDescription>
                </CardHeader>
                <CardContent>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>{t('common.table.name')}</TableHead>
                                <TableHead>{t('agentSettings.table.configStatus')}</TableHead>
                                <TableHead>{t('agentSettings.table.lastUpdate')}</TableHead>
                                <TableHead>{t('agentSettings.table.lastError')}</TableHead>
                                <TableHead className="text-right">{t('common.table.actions')}</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {servers.map((vps: VpsListItemResponse) => (
                                <TableRow key={vps.id}>
                                    <TableCell className="font-medium">{vps.name}</TableCell>
                                    <TableCell><ConfigStatusBadge status={vps.configStatus} /></TableCell>
                                    <TableCell>{vps.lastConfigUpdateAt ? new Date(vps.lastConfigUpdateAt).toLocaleString() : t('agentSettings.status.na')}</TableCell>
                                    <TableCell className="text-destructive">{vps.lastConfigError || t('agentSettings.status.none')}</TableCell>
                                    <TableCell className="text-right space-x-2">
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => handlePreviewConfig(vps.id)}
                                            disabled={previewing === vps.id}
                                        >
                                            {previewing === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : t('common.actions.preview')}
                                        </Button>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => handlePushConfig(vps.id)}
                                            disabled={pushing === vps.id}
                                        >
                                            {pushing === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : t('common.actions.push')}
                                        </Button>
                                        {vps.configStatus === 'failed' && (
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => handleRetry(vps.id)}
                                                disabled={retrying === vps.id}
                                            >
                                                {retrying === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : t('common.actions.retry')}
                                            </Button>
                                        )}
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </CardContent>
            </Card>
        </div>
    );
};

export default GlobalSettingsPage;