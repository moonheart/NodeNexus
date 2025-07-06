import React, { useEffect, useState, useMemo, useCallback } from 'react';
import { useParams, Link } from 'react-router-dom';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea, type LegendProps } from 'recharts';
import { getVpsMetricsTimeseries, getLatestNMetrics } from '../services/metricsService';
import { getMonitorResultsByVpsId } from '../services/serviceMonitorService';
import { dismissVpsRenewalReminder } from '../services/vpsService';
import type { PerformanceMetricPoint, ServiceMonitorResult, VpsListItemResponse } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { useAuthStore } from '../store/authStore';
import EditVpsModal from '../components/EditVpsModal';
import { useShallow } from 'zustand/react/shallow';
import type { ValueType } from 'recharts/types/component/DefaultTooltipContent';
import StatCard from '../components/StatCard';
import { Server, XCircle, AlertTriangle, ArrowLeft, Cpu, MemoryStick, HardDrive, ArrowUp, ArrowDown, Pencil, BellRing, Info, BarChartHorizontal } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { getVpsStatusAppearance } from '@/utils/vpsUtils';
import { VpsTags } from '@/components/VpsTags';

// Helper to format date for XAxis
const formatDateTick = (tickItem: string) => {
  const date = new Date(tickItem);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
};

// Helper to format the label in tooltips to local time
const formatTooltipLabel = (label: string) => {
  const date = new Date(label);
  return date.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
};

// Helper to format percentage for tooltips
const formatPercentForTooltip = (value: ValueType) => {
  if (typeof value === 'number') return `${value.toFixed(2)}%`;
  return `${value}`;
};

// Helper to calculate memory usage percentage
const calculateMemoryUsagePercent = (dataPoint: Partial<PerformanceMetricPoint>): number | null => {
  if (dataPoint.memoryUsageBytes != null && dataPoint.memoryTotalBytes != null && dataPoint.memoryTotalBytes > 0) {
    return (dataPoint.memoryUsageBytes / dataPoint.memoryTotalBytes) * 100;
  }
  if (dataPoint.avgMemoryUsageBytes != null && dataPoint.maxMemoryTotalBytes != null && dataPoint.maxMemoryTotalBytes > 0) {
    return (dataPoint.avgMemoryUsageBytes / dataPoint.maxMemoryTotalBytes) * 100;
  }
  return null;
};

// Helper to format Network Speed (Bytes per second)
const formatNetworkSpeed = (bps: number | null | undefined): string => {
  if (bps == null || bps < 0) return 'N/A';
  if (bps === 0) return '0 B/s';
  const k = 1024;
  const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s'];
  if (bps < 1) return bps.toFixed(2) + ' B/s';
  const i = Math.floor(Math.log(bps) / Math.log(k));
  const index = Math.min(i, sizes.length - 1);
  return parseFloat((bps / Math.pow(k, index)).toFixed(2)) + ' ' + sizes[index];
};

// Helper to format bytes
const formatBytes = (bytes: number | null | undefined, decimals = 2): string => {
  if (bytes == null || bytes < 0) return 'N/A';
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  if (bytes < 1) return bytes.toFixed(dm) + ' Bytes';
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const index = Math.min(i, sizes.length - 1);
  return parseFloat((bytes / Math.pow(k, index)).toFixed(dm)) + ' ' + sizes[index];
};

// Helper to format uptime
const formatUptime = (totalSeconds: number | null | undefined): string => {
  if (totalSeconds == null || totalSeconds < 0) return 'N/A';
  if (totalSeconds === 0) return '0 seconds';
  const days = Math.floor(totalSeconds / (3600 * 24));
  const hours = Math.floor((totalSeconds % (3600 * 24)) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  let uptimeString = '';
  if (days > 0) uptimeString += `${days}d `;
  if (hours > 0) uptimeString += `${hours}h `;
  if (minutes > 0) uptimeString += `${minutes}m `;
  if (seconds > 0 || uptimeString === '') uptimeString += `${seconds}s`;
  return uptimeString.trim();
};

const formatTrafficBillingRule = (rule: string | null | undefined): string => {
  if (!rule) return '未设置';
  switch (rule) {
    case 'sum_in_out': return '双向流量 (IN + OUT)';
    case 'out_only': return '出站流量 (OUT Only)';
    case 'max_in_out': return '单向最大值 (Max(IN, OUT))';
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
    return 'Invalid Date';
  }
};

const formatRenewalCycle = (cycle?: string | null, customDays?: number | null): string => {
  if (!cycle) return '未设置';
  switch (cycle) {
    case 'monthly': return '每月';
    case 'quarterly': return '每季度';
    case 'semi_annually': return '每半年';
    case 'annually': return '每年';
    case 'biennially': return '每两年';
    case 'triennially': return '每三年';
    case 'custom_days': return customDays ? `每 ${customDays} 天` : '自定义天数 (未指定天数)';
    default: return cycle;
  }
};

const formatBoolean = (value?: boolean | null): string => {
  if (value === null || typeof value === 'undefined') return '未设置';
  return value ? '是' : '否';
};

const TIME_RANGE_OPTIONS = [
  { label: '实时', value: 'realtime' as const },
  { label: '1H', value: '1h' as const },
  { label: '6H', value: '6h' as const },
  { label: '24H', value: '24h' as const },
  { label: '7D', value: '7d' as const },
];
type TimeRangeOption = typeof TIME_RANGE_OPTIONS[number]['value'];

const VpsDetailPage: React.FC = () => {
  const { vpsId } = useParams<{ vpsId: string }>();
  const { isAuthenticated } = useAuthStore();

  const { servers, connectionStatus, isLoading } = useServerListStore(useShallow(state => ({
    servers: state.servers,
    connectionStatus: state.connectionStatus,
    isLoading: state.isLoading,
  })));

  const vpsDetail = useMemo(() => {
    if (!vpsId) return null;
    const numericVpsId = parseInt(vpsId, 10);
    return servers.find(server => server.id === numericVpsId) || null;
  }, [vpsId, servers]);

  const [cpuData, setCpuData] = useState<PerformanceMetricPoint[]>([]);
  const [memoryData, setMemoryData] = useState<PerformanceMetricPoint[]>([]);
  const [networkData, setNetworkData] = useState<PerformanceMetricPoint[]>([]);
  const [diskIoData, setDiskIoData] = useState<PerformanceMetricPoint[]>([]);
  const [loadingChartMetrics, setLoadingChartMetrics] = useState(true);
  const [chartError, setChartError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');

  const handleSetSelectedTimeRange = useCallback((value: TimeRangeOption) => {
    if (value) setSelectedTimeRange(value);
  }, []);

  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [isDismissingReminder, setIsDismissingReminder] = useState(false);
  const [dismissReminderError, setDismissReminderError] = useState<string | null>(null);
  const [dismissReminderSuccess, setDismissReminderSuccess] = useState<string | null>(null);

  const [monitorResults, setMonitorResults] = useState<ServiceMonitorResult[]>([]);
  const [loadingMonitors, setLoadingMonitors] = useState(true);
  const [monitorError, setMonitorError] = useState<string | null>(null);

  const handleVpsUpdated = () => {
    console.log('VPS updated, store should refresh via WebSocket.');
    setIsEditModalOpen(false);
  };

  const handleDismissReminder = async () => {
    if (!vpsDetail || !vpsDetail.id) return;
    setIsDismissingReminder(true);
    setDismissReminderError(null);
    setDismissReminderSuccess(null);
    try {
      await dismissVpsRenewalReminder(vpsDetail.id);
      setDismissReminderSuccess('续费提醒已成功清除。');
    } catch (error: unknown) {
      console.error('Failed to dismiss reminder:', error);
      let errorMessage = '清除提醒失败。';
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
    ? "WebSocket 连接错误。"
    : (connectionStatus === 'connected' && !vpsDetail && !isLoading ? "未找到 VPS 详细信息。" : null);

  const timeRangeToMillis: Record<Exclude<TimeRangeOption, 'realtime'>, number> = { '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 };

  useEffect(() => {
    if (!vpsId || !isAuthenticated) {
      setLoadingChartMetrics(false);
      return;
    }
    let isMounted = true;
    const fetchChartMetricsData = async () => {
      setLoadingChartMetrics(true);
      setChartError(null);
      try {
        let metrics: PerformanceMetricPoint[];
        if (selectedTimeRange === 'realtime') {
          metrics = await getLatestNMetrics(vpsId, 300);
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
          const intervalSeconds = Math.round(timeRangeToMillis[selectedTimeRange] / 1000 / 300);
          const interval = `${intervalSeconds}s`;
          metrics = await getVpsMetricsTimeseries(vpsId, startTime.toISOString(), endTime.toISOString(), interval);
        }

        if (!isMounted) return;

        const cpuPoints: PerformanceMetricPoint[] = [];
        const memoryPoints: PerformanceMetricPoint[] = [];
        const networkPoints: PerformanceMetricPoint[] = [];
        const diskIoPoints: PerformanceMetricPoint[] = [];

        metrics.forEach(point => {
          const cpuValue = point.avgCpuUsagePercent ?? point.cpuUsagePercent;
          if (cpuValue != null) cpuPoints.push({ ...point, cpuUsagePercent: cpuValue });
          
          const memoryUsagePercentValue = calculateMemoryUsagePercent(point);
          if (memoryUsagePercentValue != null) memoryPoints.push({ ...point, memoryUsagePercent: memoryUsagePercentValue });
          
          const rxBps = point.avgNetworkRxInstantBps ?? point.networkRxInstantBps;
          const txBps = point.avgNetworkTxInstantBps ?? point.networkTxInstantBps;
          if (rxBps != null || txBps != null) {
            networkPoints.push({ ...point, avgNetworkRxInstantBps: rxBps, avgNetworkTxInstantBps: txBps });
          }

          const readBps = point.avgDiskIoReadBps ?? point.diskIoReadBps;
          const writeBps = point.avgDiskIoWriteBps ?? point.diskIoWriteBps;
          if (readBps != null || writeBps != null) {
            diskIoPoints.push({ ...point, avgDiskIoReadBps: readBps, avgDiskIoWriteBps: writeBps });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
        setNetworkData(networkPoints);
        setDiskIoData(diskIoPoints);
      } catch (err) {
        console.error('Failed to fetch chart metrics:', err);
        if (isMounted) setChartError('无法加载图表数据。');
      } finally {
        if (isMounted) setLoadingChartMetrics(false);
      }
    };
    fetchChartMetricsData();
    return () => { isMounted = false; };
  }, [vpsId, selectedTimeRange, isAuthenticated]);

  useEffect(() => {
    if (!vpsId || !isAuthenticated) {
      setLoadingMonitors(false);
      return;
    };

    const fetchMonitorData = async () => {
      setLoadingMonitors(true);
      setMonitorError(null);
      try {
        let results: ServiceMonitorResult[];
        if (selectedTimeRange === 'realtime') {
          console.log(`[VpsDetailPage] Fetching REALTIME monitor data for vpsId: ${vpsId} with limit: 300`);
          // For realtime, fetch the last 300 data points instead of a fixed time window
          results = await getMonitorResultsByVpsId(vpsId, undefined, undefined, 300);
          console.log(`[VpsDetailPage] REALTIME monitor data received for vpsId: ${vpsId}`, { count: results.length, results });
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
          console.log(`[VpsDetailPage] Fetching HISTORICAL monitor data for vpsId: ${vpsId}`, { timeRange: selectedTimeRange, startTime: startTime.toISOString(), endTime: endTime.toISOString() });
          results = await getMonitorResultsByVpsId(vpsId, startTime.toISOString(), endTime.toISOString());
          console.log(`[VpsDetailPage] HISTORICAL monitor data received for vpsId: ${vpsId}`, { count: results.length, results });
        }
        setMonitorResults(results);
      } catch (err) {
        console.error('Failed to fetch service monitor results:', err);
        setMonitorError('无法加载服务监控数据。');
      } finally {
        setLoadingMonitors(false);
      }
    };

    fetchMonitorData();
  }, [vpsId, selectedTimeRange, isAuthenticated]);

  useEffect(() => {
    if (selectedTimeRange !== 'realtime' || !vpsDetail?.latestMetrics || !vpsId || !isAuthenticated) {
      return;
    }

    const newMetrics = vpsDetail.latestMetrics;
    const newTime = newMetrics.time;
    const numericVpsId = parseInt(vpsId, 10);

    const appendAndTrim = <T extends { time: string }>(prevData: T[], newDataPoint: T): T[] => {
      if (prevData.some(p => p.time === newDataPoint.time)) {
        return prevData;
      }
      const combined = [...prevData, newDataPoint].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
      return combined.length > 300 ? combined.slice(combined.length - 300) : combined;
    };

    if (newMetrics.cpuUsagePercent != null) {
      setCpuData(prev => appendAndTrim(prev, { time: newTime, vpsId: numericVpsId, cpuUsagePercent: newMetrics.cpuUsagePercent } as PerformanceMetricPoint));
    }
    
    const memoryUsagePercentValue = calculateMemoryUsagePercent(newMetrics);
    if (memoryUsagePercentValue != null) {
      setMemoryData(prev => appendAndTrim(prev, { time: newTime, vpsId: numericVpsId, memoryUsagePercent: memoryUsagePercentValue, memoryUsageBytes: newMetrics.memoryUsageBytes, memoryTotalBytes: newMetrics.memoryTotalBytes } as PerformanceMetricPoint));
    }

    if (newMetrics.networkRxInstantBps != null || newMetrics.networkTxInstantBps != null) {
      setNetworkData(prev => appendAndTrim(prev, { time: newTime, vpsId: numericVpsId, avgNetworkRxInstantBps: newMetrics.networkRxInstantBps, avgNetworkTxInstantBps: newMetrics.networkTxInstantBps } as PerformanceMetricPoint));
    }

    if (newMetrics.diskIoReadBps != null || newMetrics.diskIoWriteBps != null) {
      setDiskIoData(prev => appendAndTrim(prev, { time: newTime, vpsId: numericVpsId, avgDiskIoReadBps: newMetrics.diskIoReadBps, avgDiskIoWriteBps: newMetrics.diskIoWriteBps } as PerformanceMetricPoint));
    }
  }, [vpsDetail?.latestMetrics?.time, vpsId, selectedTimeRange, isAuthenticated]);

  if (isLoadingPage) {
    return <div className="flex justify-center items-center h-64"><p>正在加载服务器详情...</p></div>;
  }

  if (pageError) {
    return (
      <div className="text-center py-10">
        <XCircle className="w-16 h-16 text-destructive mx-auto mb-4" />
        <p className="text-xl text-destructive-foreground bg-destructive p-4 rounded-lg">{pageError}</p>
        <Button asChild className="mt-6">
          <Link to={isAuthenticated ? "/servers" : "/"}><ArrowLeft className="w-4 h-4 mr-2" />返回仪表盘</Link>
        </Button>
      </div>
    );
  }

  if (!vpsDetail) {
    return <div className="text-center text-muted-foreground">服务器数据不可用。</div>;
  }

  const { icon: StatusIcon, variant: statusVariant } = getVpsStatusAppearance(vpsDetail.status);
  const { latestMetrics: metrics, metadata } = vpsDetail;
  const memUsed = metrics?.memoryUsageBytes ?? 0;
  const memTotal = metrics?.memoryTotalBytes ?? 0;
  const diskUsed = metrics?.diskUsedBytes ?? 0;
  const diskTotal = metrics?.diskTotalBytes ?? 0;

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
    <div className="p-4 md:p-6 lg:p-8 space-y-6">
      {isAuthenticated && vpsDetail && (
        <EditVpsModal isOpen={isEditModalOpen} onClose={() => setIsEditModalOpen(false)} vps={vpsDetail as VpsListItemResponse} allVps={servers} onVpsUpdated={handleVpsUpdated} />
      )}

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
                <Button variant="outline" size="sm" onClick={() => setIsEditModalOpen(true)}>
                  <Pencil className="w-4 h-4 mr-1.5" /> 编辑
                </Button>
              )}
              <Button variant="outline" size="sm" asChild>
                <Link to={isAuthenticated ? "/servers" : "/"}><ArrowLeft className="w-4 h-4 mr-1.5" /> 返回</Link>
              </Button>
            </div>
          </div>
        </CardHeader>
      </Card>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
        <StatCard title="CPU 使用率" value={metrics?.cpuUsagePercent?.toFixed(1) ?? 'N/A'} unit="%" icon={<Cpu />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? "离线" : `${metadata?.cpu_static_info?.brand || ''}`} />
        <StatCard title="内存使用率" value={memTotal > 0 ? ((memUsed / memTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<MemoryStick />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? "离线" : `${formatBytes(memUsed)} / ${formatBytes(memTotal)}`} />
        <StatCard title="磁盘使用率" value={diskTotal > 0 ? ((diskUsed / diskTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<HardDrive />} valueClassName="text-primary" description={vpsDetail.status === 'offline' ? "离线" : `${formatBytes(diskUsed)} / ${formatBytes(diskTotal)}`} />
        <StatCard title="上传" value={formatNetworkSpeed(metrics?.networkTxInstantBps)} icon={<ArrowUp />} valueClassName="text-primary" description="当前出站" />
        <StatCard title="下载" value={formatNetworkSpeed(metrics?.networkRxInstantBps)} icon={<ArrowDown />} valueClassName="text-primary" description="当前入站" />
        <StatCard title="运行时间" value={formatUptime(metrics?.uptimeSeconds)} icon={<AlertTriangle />} valueClassName="text-primary" description="当前会话" />
      </div>

      {isAuthenticated && vpsDetail.trafficBillingRule && (
        <Card>
          <CardHeader><CardTitle>流量信息</CardTitle></CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
            <InfoBlock title="流量限额" value={trafficLimit != null ? formatBytes(trafficLimit) : '未设置'} />
            <InfoBlock title="计费规则" value={formatTrafficBillingRule(billingRule)} />
            <InfoBlock title="下次重置时间" value={formatTrafficDate(vpsDetail.nextTrafficResetAt)} />
            <InfoBlock title="本周期已用 (总计)" value={formatBytes(totalUsedTraffic)} />
            {trafficLimit != null && <InfoBlock title="本周期剩余" value={trafficRemaining != null ? formatBytes(trafficRemaining) : 'N/A'} />}
            {trafficUsagePercent != null && <InfoBlock title="使用率" value={`${trafficUsagePercent.toFixed(2)}%`} />}
            <InfoBlock title="本周期 RX (入站)" value={formatBytes(currentRx)} />
            <InfoBlock title="本周期 TX (出站)" value={formatBytes(currentTx)} />
            <InfoBlock title="上次重置时间" value={formatTrafficDate(vpsDetail.trafficLastResetAt)} />
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <div className="flex flex-col sm:flex-row justify-between items-center gap-4">
            <CardTitle>性能指标</CardTitle>
            <ToggleGroup type="single" value={selectedTimeRange} onValueChange={handleSetSelectedTimeRange} aria-label="Time range">
              {TIME_RANGE_OPTIONS.map(period => (
                <ToggleGroupItem key={period.value} value={period.value}>{period.label}</ToggleGroupItem>
              ))}
            </ToggleGroup>
          </div>
        </CardHeader>
        <CardContent>
          {chartError && <p className="text-destructive text-center">{chartError}</p>}
          {loadingChartMetrics ? <div className="h-72 flex justify-center items-center"><p>正在加载图表...</p></div> : (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
              <ChartComponent title="CPU 使用率 (%)" data={cpuData} dataKey="cpuUsagePercent" stroke="#8884d8" yDomain={[0, 100]} />
              <ChartComponent title="内存使用率 (%)" data={memoryData} dataKey="memoryUsagePercent" stroke="#82ca9d" yDomain={[0, 100]} />
              <NetworkChartComponent data={networkData} />
              <DiskIoChartComponent data={diskIoData} />
            </div>
          )}
        </CardContent>
      </Card>

      {isAuthenticated && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center">
              <BarChartHorizontal className="w-6 h-6 mr-2 text-primary" />
              服务监控
            </CardTitle>
          </CardHeader>
          <CardContent>
            {monitorError && <p className="text-destructive text-center">{monitorError}</p>}
            {loadingMonitors ? <div className="h-72 flex justify-center items-center"><p>正在加载监控图表...</p></div> : (
              <ServiceMonitorChartComponent results={monitorResults} />
            )}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader><CardTitle>系统信息</CardTitle></CardHeader>
        <CardContent className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
          <InfoBlock title="主机名" value={`${metadata?.hostname || 'N/A'}`} />
          <InfoBlock title="操作系统" value={`${metadata?.os_name || 'N/A'} (${metadata?.long_os_version || metadata?.os_version_detail || 'N/A'})`} />
          <InfoBlock title="发行版 ID" value={`${metadata?.distribution_id || 'N/A'}`} />
          <InfoBlock title="内核版本" value={`${metadata?.kernel_version || 'N/A'}`} />
          <InfoBlock title="架构" value={`${metadata?.arch || 'N/A'}`} />
          <InfoBlock title="CPU 品牌" value={`${metadata?.cpu_static_info?.brand || 'N/A'}`} />
          <InfoBlock title="CPU 名称" value={`${metadata?.cpu_static_info?.name || 'N/A'}`} />
          <InfoBlock title="CPU 频率" value={metadata?.cpu_static_info?.frequency ? `${metadata.cpu_static_info.frequency} MHz` : 'N/A'} />
          <InfoBlock title="CPU 厂商 ID" value={`${metadata?.cpu_static_info?.vendorId || 'N/A'}`} />
          <InfoBlock title="物理核心数" value={`${metadata?.physical_core_count ?? 'N/A'}`} />
          <InfoBlock title="总内存" value={formatBytes(metadata?.total_memory_bytes ?? metrics?.memoryTotalBytes)} />
          <InfoBlock title="总交换空间" value={formatBytes(metadata?.total_swap_bytes)} />
          <InfoBlock title="总磁盘空间" value={formatBytes(metrics?.diskTotalBytes)} />
        </CardContent>
      </Card>

      {isAuthenticated && (vpsDetail.renewalCycle || vpsDetail.nextRenewalDate || vpsDetail.paymentMethod) && (
        <Card>
          <CardHeader>
            <div className="flex justify-between items-center">
              <CardTitle className="flex items-center">
                <Info className="w-6 h-6 mr-2 text-primary" />
                续费信息
              </CardTitle>
              {vpsDetail.reminderActive && (
                <Button variant="secondary" size="sm" onClick={handleDismissReminder} disabled={isDismissingReminder}>
                  <BellRing className="w-4 h-4 mr-1.5" />
                  {isDismissingReminder ? '清除中...' : '清除提醒'}
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent>
            {dismissReminderError && <p className="text-sm text-destructive bg-destructive/10 p-2 rounded-md mb-4">{dismissReminderError}</p>}
            {dismissReminderSuccess && <p className="text-sm text-success bg-success/10 p-2 rounded-md mb-4">{dismissReminderSuccess}</p>}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
              <InfoBlock title="续费周期" value={formatRenewalCycle(vpsDetail.renewalCycle, vpsDetail.renewalCycleCustomDays)} />
              <InfoBlock title="续费价格" value={vpsDetail.renewalPrice != null ? `${vpsDetail.renewalPrice} ${vpsDetail.renewalCurrency || ''}`.trim() : '未设置'} />
              <InfoBlock title="下次续费日期" value={formatTrafficDate(vpsDetail.nextRenewalDate)} />
              <InfoBlock title="上次续费日期" value={formatTrafficDate(vpsDetail.lastRenewalDate)} />
              <InfoBlock title="服务开始日期" value={formatTrafficDate(vpsDetail.serviceStartDate)} />
              <InfoBlock title="支付方式" value={vpsDetail.paymentMethod || '未设置'} />
              <InfoBlock title="自动续费" value={formatBoolean(vpsDetail.autoRenewEnabled)} />
              <InfoBlock title="提醒状态" value={formatBoolean(vpsDetail.reminderActive)} />
              {vpsDetail.renewalNotes && (
                <div className="md:col-span-2 lg:col-span-3">
                  <InfoBlock title="续费备注" value={vpsDetail.renewalNotes} />
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

const ChartComponent: React.FC<{ title: string, data: PerformanceMetricPoint[], dataKey: keyof PerformanceMetricPoint, stroke: string, yDomain: [number, number] }> = React.memo(({ title, data, dataKey, stroke, yDomain }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{title}</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: -10, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
            <YAxis domain={yDomain} tick={{ fontSize: 11 }} tickFormatter={(tick) => `${tick}%`} />
            <Tooltip formatter={formatPercentForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey={dataKey} stroke={stroke} dot={false} name={title.split(' ')[0]} isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-muted-foreground pt-16">无可用数据。</p>}
  </div>
));

const NetworkChartComponent: React.FC<{ data: PerformanceMetricPoint[] }> = React.memo(({ data }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">网络速度</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
            <YAxis tickFormatter={formatNetworkSpeed} width={80} tick={{ fontSize: 11 }} />
            <Tooltip formatter={(value: ValueType) => formatNetworkSpeed(value as number)} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey="avgNetworkRxInstantBps" stroke="hsl(var(--primary))" dot={false} name="下载" isAnimationActive={false} />
            <Line type="monotone" dataKey="avgNetworkTxInstantBps" stroke="hsl(var(--secondary-foreground))" dot={false} name="上传" isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-muted-foreground pt-16">无可用数据。</p>}
  </div>
));

const DiskIoChartComponent: React.FC<{ data: PerformanceMetricPoint[] }> = React.memo(({ data }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">磁盘 I/O 速度</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
            <YAxis tickFormatter={formatNetworkSpeed} width={80} tick={{ fontSize: 11 }} />
            <Tooltip formatter={(value: ValueType) => formatNetworkSpeed(value as number)} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey="avgDiskIoReadBps" stroke="#ff7300" dot={false} name="读取" isAnimationActive={false} />
            <Line type="monotone" dataKey="avgDiskIoWriteBps" stroke="#387908" dot={false} name="写入" isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-muted-foreground pt-16">无可用数据。</p>}
  </div>
));

const formatLatencyForTooltip = (value: ValueType) => {
  if (typeof value === 'number') return `${value.toFixed(0)} ms`;
  return `${value}`;
};

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];

const ServiceMonitorChartComponent: React.FC<{ results: ServiceMonitorResult[] }> = React.memo(({ results }) => {
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});

  const { chartData, monitorLines, downtimeAreas } = useMemo(() => {
    if (!results || results.length === 0) {
      return { chartData: [], monitorLines: [], downtimeAreas: [] };
    }
    const sortedResults = [...results].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
    const groupedByMonitorId = sortedResults.reduce((acc, result) => {
      const monitorId = result.monitorId;
      if (!acc[monitorId]) acc[monitorId] = [];
      acc[monitorId].push(result);
      return acc;
    }, {} as Record<number, ServiceMonitorResult[]>);

    const monitorLines: { dataKey: string; name: string; stroke: string }[] = [];
    let colorIndex = 0;
    for (const monitorId in groupedByMonitorId) {
      if (Object.prototype.hasOwnProperty.call(groupedByMonitorId, monitorId)) {
        const firstResult = groupedByMonitorId[monitorId][0];
        const monitorName = firstResult.monitorName || `Monitor #${monitorId}`;
        const color = AGENT_COLORS[colorIndex % AGENT_COLORS.length];
        monitorLines.push({ dataKey: `monitor_${monitorId}`, name: monitorName, stroke: color });
        colorIndex++;
      }
    }

    const timePoints = [...new Set(sortedResults.map(r => new Date(r.time).toISOString()))].sort();
    const chartData = timePoints.map(time => {
      const point: { time: string; [key: string]: number | null | string } = { time };
      for (const monitorId in groupedByMonitorId) {
        const dataKey = `monitor_${monitorId}`;
        const resultForTime = groupedByMonitorId[monitorId].find(r => new Date(r.time).toISOString() === time);
        point[dataKey] = resultForTime && resultForTime.isUp ? resultForTime.latencyMs : null;
      }
      return point;
    });

    const areas: { x1: string, x2: string }[] = [];
    let downtimeStart: string | null = null;
    for (let i = 0; i < timePoints.length; i++) {
      const time = timePoints[i];
      const isAnyDown = Object.values(groupedByMonitorId).some(monitorResults => monitorResults.some(r => new Date(r.time).toISOString() === time && !r.isUp));
      if (isAnyDown && !downtimeStart) {
        downtimeStart = time;
      } else if (!isAnyDown && downtimeStart) {
        const prevTime = i > 0 ? timePoints[i-1] : downtimeStart;
        areas.push({ x1: downtimeStart, x2: prevTime });
        downtimeStart = null;
      }
    }
    if (downtimeStart) {
      areas.push({ x1: downtimeStart, x2: timePoints[timePoints.length - 1] });
    }
    return { chartData, monitorLines, downtimeAreas: areas };
  }, [results]);

  const handleLegendClick: LegendProps['onClick'] = (data) => {
    const dataKey = data.dataKey as string;
    if (typeof dataKey === 'string') {
      setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
    }
  };

  const renderLegendText: LegendProps['formatter'] = (value, entry) => {
    const { color, dataKey } = entry;
    const isHidden = typeof dataKey === 'string' && hiddenLines[dataKey];
    return <span style={{ color: isHidden ? '#A0A0A0' : color || '#000', cursor: 'pointer' }}>{value}</span>;
  };

  if (results.length === 0) {
    return <p className="text-center text-muted-foreground pt-16">此 VPS 无可用的服务监控数据。</p>;
  }

  return (
    <div className="h-80">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
          <YAxis tickFormatter={(tick) => `${tick} ms`} width={80} tick={{ fontSize: 11 }} />
          <Tooltip formatter={formatLatencyForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
          <Legend wrapperStyle={{ fontSize: '0.8rem' }} onClick={handleLegendClick} formatter={renderLegendText} />
          {downtimeAreas.map((area, index) => (
            <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="hsl(var(--destructive))" fillOpacity={0.15} ifOverflow="visible" />
          ))}
          {monitorLines.map((line) => (
            <Line key={line.dataKey} type="monotone" dataKey={line.dataKey} name={line.name} stroke={hiddenLines[line.dataKey] ? 'transparent' : line.stroke} dot={false} connectNulls={true} strokeWidth={2} isAnimationActive={false} />
          ))}
        </LineChart>
      </ResponsiveContainer>
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