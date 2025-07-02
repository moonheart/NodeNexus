import React, { useState, useEffect, useCallback } from 'react';
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

const ConfigStatusBadge: React.FC<{ status: string }> = ({ status }) => {
    const statusMap: { [key: string]: { text: string; variant: "default" | "destructive" | "outline" | "secondary" | "success" | "warning" } } = {
        synced: { text: 'Synced', variant: 'success' },
        pending: { text: 'Pending', variant: 'warning' },
        failed: { text: 'Failed', variant: 'destructive' },
        unknown: { text: 'Unknown', variant: 'secondary' },
    };
    const { text, variant } = statusMap[status] || statusMap.unknown;
    return (
        <Badge variant={variant}>{text}</Badge>
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
            const errorMessage = err instanceof Error ? err.message : 'Failed to load global configuration.';
            setError(errorMessage);
            toast.error(errorMessage);
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
        const toastId = toast.loading('Saving configuration...');
        try {
            await updateGlobalConfig(config);
            toast.success('Configuration saved successfully! It will be pushed to relevant agents.', { id: toastId });
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to save configuration.';
            setError(errorMessage);
            console.error(err);
            toast.error(`Error: ${errorMessage}`, { id: toastId });
        } finally {
            setIsSaving(false);
        }
    };

    const handleRetry = async (vpsId: number) => {
        setRetrying(vpsId);
        const toastId = toast.loading(`Retrying config push for VPS ID: ${vpsId}`);
        try {
            await retryConfigPush(vpsId);
            toast.success(`Successfully initiated config push retry for VPS ID: ${vpsId}`, { id: toastId });
        } catch (err) {
            console.error(`Failed to retry config push for VPS ID: ${vpsId}`, err);
            toast.error(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`, { id: toastId });
        } finally {
            setRetrying(null);
        }
    };

    const handlePushConfig = async (vpsId: number) => {
        setPushing(vpsId);
        const toastId = toast.loading(`Triggering config push for VPS ID: ${vpsId}`);
        try {
            await pushConfig(vpsId);
            toast.success(`Configuration push triggered for VPS ID: ${vpsId}`, { id: toastId });
        } catch (err) {
            console.error(`Failed to trigger config push for VPS ID: ${vpsId}`, err);
            toast.error(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`, { id: toastId });
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
            toast.error(`Error: ${err instanceof Error ? err.message : 'Failed to preview config'}`);
        } finally {
            setPreviewing(null);
        }
    };

    if (isLoading) {
        return <div className="flex items-center justify-center h-full"><RefreshCwIcon className="h-8 w-8 animate-spin" /></div>;
    }

    if (error) {
        return (
            <Alert variant="destructive">
                <AlertTitle>Error</AlertTitle>
                <AlertDescription>{error}</AlertDescription>
            </Alert>
        );
    }

    return (
        <div className="space-y-6">
            <Dialog open={isPreviewModalOpen} onOpenChange={setIsPreviewModalOpen}>
                <DialogContent className="sm:max-w-[600px]">
                    <DialogHeader>
                        <DialogTitle>Configuration Preview</DialogTitle>
                    </DialogHeader>
                    <div className="mt-2">
                        <pre className="bg-muted p-4 rounded-md text-sm overflow-auto max-h-[60vh]">
                            <code>{previewContent}</code>
                        </pre>
                    </div>
                    <DialogFooter>
                        <Button onClick={() => setIsPreviewModalOpen(false)}>Close</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>

            <Card>
                <CardHeader>
                    <CardTitle>Global Agent Configuration</CardTitle>
                    <CardDescription>This configuration applies to all agents unless overridden by a specific VPS setting.</CardDescription>
                </CardHeader>
                <CardContent>
                    {config && (
                        <form onSubmit={handleSave}>
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div className="space-y-2">
                                    <Label htmlFor="metricsCollectIntervalSeconds">Metrics Collect Interval (s)</Label>
                                    <Input id="metricsCollectIntervalSeconds" name="metricsCollectIntervalSeconds" type="number" value={config.metricsCollectIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="metricsUploadBatchMaxSize">Metrics Upload Batch Max Size</Label>
                                    <Input id="metricsUploadBatchMaxSize" name="metricsUploadBatchMaxSize" type="number" value={config.metricsUploadBatchMaxSize} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="metricsUploadIntervalSeconds">Metrics Upload Interval (s)</Label>
                                    <Input id="metricsUploadIntervalSeconds" name="metricsUploadIntervalSeconds" type="number" value={config.metricsUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="dockerInfoCollectIntervalSeconds">Docker Info Collect Interval (s)</Label>
                                    <Input id="dockerInfoCollectIntervalSeconds" name="dockerInfoCollectIntervalSeconds" type="number" value={config.dockerInfoCollectIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="dockerInfoUploadIntervalSeconds">Docker Info Upload Interval (s)</Label>
                                    <Input id="dockerInfoUploadIntervalSeconds" name="dockerInfoUploadIntervalSeconds" type="number" value={config.dockerInfoUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="genericMetricsUploadBatchMaxSize">Generic Metrics Upload Batch Max Size</Label>
                                    <Input id="genericMetricsUploadBatchMaxSize" name="genericMetricsUploadBatchMaxSize" type="number" value={config.genericMetricsUploadBatchMaxSize} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="genericMetricsUploadIntervalSeconds">Generic Metrics Upload Interval (s)</Label>
                                    <Input id="genericMetricsUploadIntervalSeconds" name="genericMetricsUploadIntervalSeconds" type="number" value={config.genericMetricsUploadIntervalSeconds} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="logLevel">Log Level</Label>
                                    <Input id="logLevel" name="logLevel" type="text" value={config.logLevel} onChange={handleInputChange} />
                                </div>
                                <div className="space-y-2">
                                    <Label htmlFor="heartbeatIntervalSeconds">Heartbeat Interval (s)</Label>
                                    <Input id="heartbeatIntervalSeconds" name="heartbeatIntervalSeconds" type="number" value={config.heartbeatIntervalSeconds} onChange={handleInputChange} />
                                </div>
                            </div>
                            <div className="mt-6 flex justify-end">
                                <Button type="submit" disabled={isSaving}>
                                    {isSaving && <RefreshCwIcon className="mr-2 h-4 w-4 animate-spin" />}
                                    {isSaving ? 'Saving...' : 'Save Global Config'}
                                </Button>
                            </div>
                        </form>
                    )}
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>VPS Configuration Status</CardTitle>
                    <CardDescription>Monitor the configuration sync status for each connected VPS.</CardDescription>
                </CardHeader>
                <CardContent>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>Name</TableHead>
                                <TableHead>Config Status</TableHead>
                                <TableHead>Last Update</TableHead>
                                <TableHead>Last Error</TableHead>
                                <TableHead className="text-right">Actions</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {servers.map((vps: VpsListItemResponse) => (
                                <TableRow key={vps.id}>
                                    <TableCell className="font-medium">{vps.name}</TableCell>
                                    <TableCell><ConfigStatusBadge status={vps.configStatus} /></TableCell>
                                    <TableCell>{vps.lastConfigUpdateAt ? new Date(vps.lastConfigUpdateAt).toLocaleString() : 'N/A'}</TableCell>
                                    <TableCell className="text-destructive">{vps.lastConfigError || 'None'}</TableCell>
                                    <TableCell className="text-right space-x-2">
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => handlePreviewConfig(vps.id)}
                                            disabled={previewing === vps.id}
                                        >
                                            {previewing === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : 'Preview'}
                                        </Button>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            onClick={() => handlePushConfig(vps.id)}
                                            disabled={pushing === vps.id}
                                        >
                                            {pushing === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : 'Push'}
                                        </Button>
                                        {vps.configStatus === 'failed' && (
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => handleRetry(vps.id)}
                                                disabled={retrying === vps.id}
                                            >
                                                {retrying === vps.id ? <RefreshCwIcon className="h-4 w-4 animate-spin" /> : 'Retry'}
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