import React, { useEffect, useState, useMemo, useCallback } from 'react';
import { useParams, Link } from 'react-router-dom';
import { dismissVpsRenewalReminder } from '../services/vpsService';
import type { VpsListItemResponse } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { useAuthStore } from '../store/authStore';
import EditVpsModal from '../components/EditVpsModal';
import { useShallow } from 'zustand/react/shallow';
import StatCard from '../components/StatCard';
import { Server, XCircle, AlertTriangle, ArrowLeft, Cpu, MemoryStick, HardDrive, ArrowUp, ArrowDown, Pencil, BellRing, Info, BarChartHorizontal } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getVpsStatusAppearance, formatBytesForDisplay, formatNetworkSpeed, formatUptime } from '@/utils/vpsUtils';
import { VpsTags } from '@/components/VpsTags';
import { useTranslation } from 'react-i18next';
import type { TimeRangeValue } from '@/components/TimeRangeSelector';
import UnifiedMetricChart from '@/components/metric/UnifiedMetricChart';
import { useVpsPerformanceMetrics } from '@/hooks/useVpsPerformanceMetrics';
import type { ChartViewMode } from '@/hooks/useMetrics';

const VpsDetailLayout: React.FC<{
  vpsDetail: VpsListItemResponse;
  isAuthenticated: boolean;
  handleOpenEditModal: () => void;
  children: React.ReactNode;
}> = ({ vpsDetail, isAuthenticated, handleOpenEditModal, children }) => {
  const { t } = useTranslation();
  const metrics = useServerListStore(state => state.latestMetrics[vpsDetail.id]);
  const { icon: StatusIcon, variant: statusVariant } = getVpsStatusAppearance(vpsDetail.status);

  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-6">
      <Card>
        <CardHeader className="flex flex-col sm:flex-row justify-between items-start gap-4">
          <div>
            <div className="flex items-center mb-2">
              <Server className="w-8 h-8 mr-3 text-primary flex-shrink-0" />
              <h1 className="text-3xl font-bold">{vpsDetail.name}</h1>
            </div>
            {vpsDetail.ipAddress && <p className="text-muted-foreground mt-1 ml-11">IP: {vpsDetail.ipAddress}</p>}
            <VpsTags tags={vpsDetail.tags} className="mt-2 ml-11" />
          </div>
          <div className="mt-4 sm:mt-0 sm:text-right space-y-2 w-full sm:w-auto">
            <Badge variant={statusVariant} className="text-sm py-1.5 px-4">
              <StatusIcon className="w-4 h-4 mr-2" />
              {vpsDetail.status.toUpperCase()}
            </Badge>
            <div className="flex items-center space-x-2 justify-end">
              {isAuthenticated && (
                <Button variant="outline" size="sm" onClick={handleOpenEditModal}>
                  <Pencil className="w-4 h-4 mr-1.5" /> {t('common.actions.edit')}
                </Button>
              )}
              <Button variant="outline" size="sm" asChild>
                <Link to={isAuthenticated ? "/servers" : "/"}><ArrowLeft className="w-4 h-4 mr-1.5" /> {t('vpsDetailPage.back')}</Link>
              </Button>
            </div>
          </div>
        </CardHeader>
      </Card>

      <VpsStatCards vpsDetail={vpsDetail} metrics={metrics} />

      {children}
    </div>
  );
};

const PerformanceCharts: React.FC<{ vpsId: number; activeTab: string }> = ({ vpsId, activeTab }) => {
  const viewMode: ChartViewMode = activeTab === 'realtime' ? 'realtime' : 'historical';
  const { data, loading } = useVpsPerformanceMetrics({
    vpsId,
    viewMode,
    timeRange: activeTab as TimeRangeValue,
  });

  const chartMetrics: { metricType: 'cpu' | 'ram' | 'network' | 'disk' }[] = [
    { metricType: 'cpu' },
    { metricType: 'ram' },
    { metricType: 'network' },
    { metricType: 'disk' },
  ];

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
      {chartMetrics.map(({ metricType }) => (
        <UnifiedMetricChart
          key={metricType}
          sourceType="vps"
          sourceId={vpsId}
          metricType={metricType}
          viewMode={viewMode}
          timeRange={activeTab as TimeRangeValue}
          data={data}
          loading={loading}
          className="h-72 w-full"
        />
      ))}
    </div>
  );
};


const VpsDetailPage: React.FC = () => {
  const { t } = useTranslation();
  const { vpsId } = useParams<{ vpsId: string }>();
  const { isAuthenticated } = useAuthStore();

  const { servers, connectionStatus, isLoading, allTags, fetchAllTags } = useServerListStore(useShallow(state => ({
    servers: state.servers,
    connectionStatus: state.connectionStatus,
    isLoading: state.isLoading,
    allTags: state.allTags,
    fetchAllTags: state.fetchAllTags,
  })));

  const vpsDetail = useMemo(() => {
    if (!vpsId) return null;
    const numericVpsId = parseInt(vpsId, 10);
    return servers.find(server => server.id === numericVpsId) || null;
  }, [vpsId, servers]);

  const [activePerformanceTab, setActivePerformanceTab] = useState<string>('realtime');
  const [activeMonitorTab, setActiveMonitorTab] = useState<string>('realtime');

  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editingModalData, setEditingModalData] = useState<{
    vps: VpsListItemResponse;
    groupOptions: { value: string; label: string }[];
    tagOptions: { id: number; name: string; color: string }[];
  } | null>(null);
  const [isDismissingReminder, setIsDismissingReminder] = useState(false);
  const [dismissReminderError, setDismissReminderError] = useState<string | null>(null);
  const [dismissReminderSuccess, setDismissReminderSuccess] = useState<string | null>(null);

  const formatTrafficBillingRule = (rule: string | null | undefined): string => {
    if (!rule) return t('vpsDetailPage.notSet');
    switch (rule) {
      case 'sum_in_out': return t('vpsDetailPage.trafficBillingRules.sumInOut');
      case 'out_only': return t('vpsDetailPage.trafficBillingRules.outOnly');
      case 'max_in_out': return t('vpsDetailPage.trafficBillingRules.maxInOut');
      default: return rule;
    }
  };

  const formatTrafficDate = (dateString: string | null | undefined): string => {
    if (!dateString) return 'N/A';
    try {
      const date = new Date(dateString);
      return date.toLocaleString([], { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
    } catch (e) {
      console.error("Error formatting traffic date:", dateString, e);
      return t('vpsDetailPage.invalidDate');
    }
  };

  const formatRenewalCycle = (cycle?: string | null, customDays?: number | null): string => {
    if (!cycle) return t('vpsDetailPage.notSet');
    switch (cycle) {
      case 'monthly': return t('vpsDetailPage.renewalCycles.monthly');
      case 'quarterly': return t('vpsDetailPage.renewalCycles.quarterly');
      case 'semi_annually': return t('vpsDetailPage.renewalCycles.semiAnnually');
      case 'annually': return t('vpsDetailPage.renewalCycles.annually');
      case 'biennially': return t('vpsDetailPage.renewalCycles.biennially');
      case 'triennially': return t('vpsDetailPage.renewalCycles.triennially');
      case 'custom_days': return customDays ? t('vpsDetailPage.renewalCycles.customDays', { count: customDays }) : t('vpsDetailPage.renewalCycles.customDaysNotSet');
      default: return cycle;
    }
  };

  const formatBoolean = (value?: boolean | null): string => {
    if (value === null || typeof value === 'undefined') return t('vpsDetailPage.notSet');
    return value ? t('vpsDetailPage.yes') : t('vpsDetailPage.no');
  };

  const handleVpsUpdated = useCallback(() => {
    console.log('VPS updated, store should refresh via WebSocket.');
    setIsEditModalOpen(false);
  }, []);

  useEffect(() => {
    fetchAllTags();
  }, [fetchAllTags]);

  const handleOpenEditModal = useCallback(() => {
    if (!vpsDetail) return;
    fetchAllTags();
    const allGroups = new Set(servers.map(v => v.group).filter((g): g is string => !!g));
    const groupOptions = [...allGroups].map(g => ({ value: g, label: g }));
    const tagOptions = allTags.map(t => ({ id: t.id, name: t.name, color: t.color }));

    setEditingModalData({
        vps: vpsDetail,
        groupOptions,
        tagOptions,
    });
    setIsEditModalOpen(true);
  }, [servers, vpsDetail, allTags, fetchAllTags]);

  const handleCloseEditModal = useCallback(() => {
    setIsEditModalOpen(false);
    setEditingModalData(null);
  }, []);

  const handleDismissReminder = async () => {
    if (!vpsDetail || !vpsDetail.id) return;
    setIsDismissingReminder(true);
    setDismissReminderError(null);
    setDismissReminderSuccess(null);
    try {
      await dismissVpsRenewalReminder(vpsDetail.id);
      setDismissReminderSuccess(t('vpsDetailPage.notifications.dismissSuccess'));
    } catch (error: unknown) {
      console.error('Failed to dismiss reminder:', error);
      let errorMessage = t('vpsDetailPage.notifications.dismissError');
      if (typeof error === 'object' && error !== null) {
        const errAsAxios = error as { response?: { data?: { error?: string } }, message?: string };
        if (errAsAxios.response?.data?.error) {
          errorMessage = errAsAxios.response.data.error;
        } else if (errAsAxios.message) {
          errorMessage = errAsAxios.message;
        }
      }
      setDismissReminderError(errorMessage);
    } finally {
      setIsDismissingReminder(false);
    }
  };

  const isLoadingPage = isLoading && !vpsDetail;
  const pageError = (connectionStatus === 'error' || connectionStatus === 'permanently_failed')
    ? t('vpsDetailPage.errors.webSocket')
    : (connectionStatus === 'connected' && !vpsDetail && !isLoading ? t('vpsDetailPage.errors.notFound') : null);

  if (isLoadingPage) {
    return <div className="flex justify-center items-center h-64"><p>{t('vpsDetailPage.loadingDetails')}</p></div>;
  }

  if (pageError) {
    return (
      <div className="text-center py-10">
        <XCircle className="w-16 h-16 text-destructive mx-auto mb-4" />
        <p className="text-xl text-destructive-foreground bg-destructive p-4 rounded-lg">{pageError}</p>
        <Button asChild className="mt-6">
          <Link to={isAuthenticated ? "/servers" : "/"}><ArrowLeft className="w-4 h-4 mr-2" />{t('vpsDetailPage.backToDashboard')}</Link>
        </Button>
      </div>
    );
  }

  if (!vpsDetail) {
    return <div className="text-center text-muted-foreground">{t('vpsDetailPage.noData')}</div>;
  }

  const { trafficLimitBytes: trafficLimit, trafficCurrentCycleRxBytes, trafficCurrentCycleTxBytes, trafficBillingRule: billingRule } = vpsDetail;
  const currentRx = trafficCurrentCycleRxBytes ?? 0;
  const currentTx = trafficCurrentCycleTxBytes ?? 0;

  let totalUsedTraffic = 0;
  if (billingRule === 'sum_in_out') totalUsedTraffic = currentRx + currentTx;
  else if (billingRule === 'out_only') totalUsedTraffic = currentTx;
  else if (billingRule === 'max_in_out') totalUsedTraffic = Math.max(currentRx, currentTx);
  else if (billingRule) totalUsedTraffic = currentRx + currentTx;

  const trafficRemaining = trafficLimit != null ? trafficLimit - totalUsedTraffic : null;
  const trafficUsagePercent = trafficLimit != null && trafficLimit > 0 ? (totalUsedTraffic / trafficLimit) * 100 : null;

  return (
    <VpsDetailLayout vpsDetail={vpsDetail} isAuthenticated={isAuthenticated} handleOpenEditModal={handleOpenEditModal}>
      {isAuthenticated && editingModalData && (
        <EditVpsModal
          isOpen={isEditModalOpen}
          onClose={handleCloseEditModal}
          vps={editingModalData.vps}
          groupOptions={editingModalData.groupOptions}
          tagOptions={editingModalData.tagOptions}
          onVpsUpdated={handleVpsUpdated}
        />
      )}

      {isAuthenticated && vpsDetail.trafficBillingRule && (
        <Card>
          <CardHeader><CardTitle>{t('vpsDetailPage.trafficInfo.title')}</CardTitle></CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
            <InfoBlock title={t('vpsDetailPage.trafficInfo.limit')} value={trafficLimit != null ? formatBytesForDisplay(trafficLimit) : t('vpsDetailPage.notSet')} />
            <InfoBlock title={t('vpsDetailPage.trafficInfo.billingRule')} value={formatTrafficBillingRule(billingRule)} />
            <InfoBlock title={t('vpsDetailPage.trafficInfo.nextResetDate')} value={formatTrafficDate(vpsDetail.nextTrafficResetAt)} />
            <InfoBlock title={t('vpsDetailPage.trafficInfo.usedThisCycleTotal')} value={formatBytesForDisplay(totalUsedTraffic)} />
            {trafficLimit != null && <InfoBlock title={t('vpsDetailPage.trafficInfo.remainingThisCycle')} value={trafficRemaining != null ? formatBytesForDisplay(trafficRemaining) : 'N/A'} />}
            {trafficUsagePercent != null && <InfoBlock title={t('vpsDetailPage.trafficInfo.usagePercentage')} value={`${trafficUsagePercent.toFixed(2)}%`} />}
            <InfoBlock title={t('vpsDetailPage.trafficInfo.rxThisCycle')} value={formatBytesForDisplay(currentRx)} />
            <InfoBlock title={t('vpsDetailPage.trafficInfo.txThisCycle')} value={formatBytesForDisplay(currentTx)} />
            <InfoBlock title={t('vpsDetailPage.trafficInfo.lastResetDate')} value={formatTrafficDate(vpsDetail.trafficLastResetAt)} />
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t('vpsDetailPage.performanceMetrics.title')}</CardTitle>
        </CardHeader>
        <CardContent>
          <Tabs value={activePerformanceTab} onValueChange={setActivePerformanceTab}>
            <div className="flex justify-end">
              <TabsList>
                <TabsTrigger value="realtime">{t('vpsDetailPage.tabs.realtime')}</TabsTrigger>
                <TabsTrigger value="1h">{t('vpsDetailPage.tabs.1h')}</TabsTrigger>
                <TabsTrigger value="6h">{t('vpsDetailPage.tabs.6h')}</TabsTrigger>
                <TabsTrigger value="1d">{t('vpsDetailPage.tabs.1d')}</TabsTrigger>
                <TabsTrigger value="7d">{t('vpsDetailPage.tabs.7d')}</TabsTrigger>
              </TabsList>
            </div>
            <PerformanceCharts vpsId={vpsDetail.id} activeTab={activePerformanceTab} />
          </Tabs>
        </CardContent>
      </Card>

      {isAuthenticated && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center">
              <BarChartHorizontal className="w-6 h-6 mr-2 text-primary" />
              {t('vpsDetailPage.serviceMonitoring.title')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Tabs value={activeMonitorTab} onValueChange={setActiveMonitorTab}>
              <div className="flex justify-end">
                <TabsList>
                  <TabsTrigger value="realtime">{t('vpsDetailPage.tabs.realtime')}</TabsTrigger>
                  <TabsTrigger value="1h">{t('vpsDetailPage.tabs.1h')}</TabsTrigger>
                  <TabsTrigger value="6h">{t('vpsDetailPage.tabs.6h')}</TabsTrigger>
                  <TabsTrigger value="1d">{t('vpsDetailPage.tabs.1d')}</TabsTrigger>
                  <TabsTrigger value="7d">{t('vpsDetailPage.tabs.7d')}</TabsTrigger>
                </TabsList>
              </div>
              <div className="mt-4 h-80">
                <UnifiedMetricChart
                  sourceType="vps"
                  sourceId={vpsDetail.id}
                  metricType="service-latency"
                  viewMode={activeMonitorTab === 'realtime' ? 'realtime' : 'historical'}
                  timeRange={activeMonitorTab as TimeRangeValue}
                  className="h-full w-full"
                />
              </div>
            </Tabs>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader><CardTitle>{t('vpsDetailPage.systemInfo.title')}</CardTitle></CardHeader>
        <CardContent className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
          <InfoBlock title={t('vpsDetailPage.systemInfo.hostname')} value={`${vpsDetail.metadata?.hostname || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.os')} value={`${vpsDetail.metadata?.os_name || 'N/A'} (${vpsDetail.metadata?.long_os_version || vpsDetail.metadata?.os_version_detail || 'N/A'})`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.distroId')} value={`${vpsDetail.metadata?.distribution_id || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.kernelVersion')} value={`${vpsDetail.metadata?.kernel_version || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.arch')} value={`${vpsDetail.metadata?.arch || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.cpuBrand')} value={`${vpsDetail.metadata?.cpu_static_info?.brand || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.cpuName')} value={`${vpsDetail.metadata?.cpu_static_info?.name || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.cpuFrequency')} value={vpsDetail.metadata?.cpu_static_info?.frequency ? `${vpsDetail.metadata.cpu_static_info.frequency} MHz` : 'N/A'} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.cpuVendorId')} value={`${vpsDetail.metadata?.cpu_static_info?.vendorId || 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.physicalCores')} value={`${vpsDetail.metadata?.physical_core_count ?? 'N/A'}`} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.totalMemory')} value={formatBytesForDisplay(vpsDetail.metadata?.total_memory_bytes)} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.totalSwap')} value={formatBytesForDisplay(vpsDetail.metadata?.total_swap_bytes)} />
          <InfoBlock title={t('vpsDetailPage.systemInfo.totalDisk')} value={formatBytesForDisplay(vpsDetail.metadata?.total_disk_bytes)} />
        </CardContent>
      </Card>

      {isAuthenticated && (vpsDetail.renewalCycle || vpsDetail.nextRenewalDate || vpsDetail.paymentMethod) && (
        <Card>
          <CardHeader>
            <div className="flex justify-between items-center">
              <CardTitle className="flex items-center">
                <Info className="w-6 h-6 mr-2 text-primary" />
                {t('vpsDetailPage.renewalInfo.title')}
              </CardTitle>
              {vpsDetail.reminderActive && (
                <Button variant="secondary" size="sm" onClick={handleDismissReminder} disabled={isDismissingReminder}>
                  <BellRing className="w-4 h-4 mr-1.5" />
                  {isDismissingReminder ? t('vpsDetailPage.renewalInfo.dismissing') : t('vpsDetailPage.renewalInfo.dismiss')}
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent>
            {dismissReminderError && <p className="text-sm text-destructive bg-destructive/10 p-2 rounded-md mb-4">{dismissReminderError}</p>}
            {dismissReminderSuccess && <p className="text-sm text-success bg-success/10 p-2 rounded-md mb-4">{dismissReminderSuccess}</p>}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
              <InfoBlock title={t('vpsDetailPage.renewalInfo.cycle')} value={formatRenewalCycle(vpsDetail.renewalCycle, vpsDetail.renewalCycleCustomDays)} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.price')} value={vpsDetail.renewalPrice != null ? `${vpsDetail.renewalPrice} ${vpsDetail.renewalCurrency || ''}`.trim() : t('vpsDetailPage.notSet')} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.nextDate')} value={formatTrafficDate(vpsDetail.nextRenewalDate)} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.lastDate')} value={formatTrafficDate(vpsDetail.lastRenewalDate)} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.startDate')} value={formatTrafficDate(vpsDetail.serviceStartDate)} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.paymentMethod')} value={vpsDetail.paymentMethod || t('vpsDetailPage.notSet')} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.autoRenew')} value={formatBoolean(vpsDetail.autoRenewEnabled)} />
              <InfoBlock title={t('vpsDetailPage.renewalInfo.reminderStatus')} value={formatBoolean(vpsDetail.reminderActive)} />
              {vpsDetail.renewalNotes && (
                <div className="md:col-span-2 lg:col-span-3">
                  <InfoBlock title={t('vpsDetailPage.renewalInfo.notes')} value={vpsDetail.renewalNotes} />
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </VpsDetailLayout>
  );
};

const VpsStatCards: React.FC<{ vpsDetail: VpsListItemResponse, metrics: ReturnType<typeof useServerListStore.getState>['latestMetrics'][number] | null }> = React.memo(({ vpsDetail, metrics }) => {
  const { t } = useTranslation();
  const memUsed = metrics?.memoryUsageBytes ?? 0;
  const memTotal = metrics?.memoryTotalBytes ?? vpsDetail.metadata?.total_memory_bytes ?? 0;
  const diskUsed = metrics?.diskUsedBytes ?? 0;
  const diskTotal = metrics?.diskTotalBytes ?? vpsDetail.metadata?.total_disk_bytes ?? 0;

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
      <StatCard title={t('vpsDetailPage.statCards.cpuUsage')} value={metrics?.cpuUsagePercent?.toFixed(1) ?? 'N/A'} unit="%" icon={<Cpu />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? t('vpsDetailPage.offline') : `${vpsDetail.metadata?.cpu_static_info?.brand || ''}`} />
      <StatCard title={t('vpsDetailPage.statCards.memoryUsage')} value={memTotal > 0 ? ((memUsed / memTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<MemoryStick />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? t('vpsDetailPage.offline') : `${formatBytesForDisplay(memUsed)} / ${formatBytesForDisplay(memTotal)}`} />
      <StatCard title={t('vpsDetailPage.statCards.diskUsage')} value={diskTotal > 0 ? ((diskUsed / diskTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<HardDrive />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? t('vpsDetailPage.offline') : `${formatBytesForDisplay(diskUsed)} / ${formatBytesForDisplay(diskTotal)}`} />
      <StatCard title={t('vpsDetailPage.statCards.upload')} value={formatNetworkSpeed(metrics?.networkTxInstantBps)} icon={<ArrowUp />} valueClassName="text-primary" description={t('vpsDetailPage.statCards.currentOutbound')} />
      <StatCard title={t('vpsDetailPage.statCards.download')} value={formatNetworkSpeed(metrics?.networkRxInstantBps)} icon={<ArrowDown />} valueClassName="text-primary" description={t('vpsDetailPage.statCards.currentInbound')} />
      <StatCard title={t('vpsDetailPage.statCards.uptime')} value={formatUptime(metrics?.uptimeSeconds)} icon={<AlertTriangle />} valueClassName="text-primary" description={t('vpsDetailPage.statCards.currentSession')} />
    </div>
  );
});

const InfoBlock: React.FC<{ title: string, value: string }> = React.memo(({ title, value }) => (
  <div className="space-y-1">
    <p className="font-medium text-muted-foreground block">{title}</p>
    <p className="text-foreground">{value}</p>
  </div>
));

export default VpsDetailPage;
