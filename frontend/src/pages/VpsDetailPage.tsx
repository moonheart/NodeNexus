import React, { useEffect, useState, useMemo, useCallback } from 'react';
import { useParams, Link } from 'react-router-dom';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea } from 'recharts';
import { getVpsMetricsTimeseries, getLatestNMetrics } from '../services/metricsService';
import { getMonitorResultsByVpsId } from '../services/serviceMonitorService';
import { dismissVpsRenewalReminder } from '../services/vpsService';
import type { PerformanceMetricPoint, ServerStatus, ServiceMonitorResult } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { useAuthStore } from '../store/authStore';
import websocketService from '../services/websocketService';
import EditVpsModal from '../components/EditVpsModal';
import { useShallow } from 'zustand/react/shallow';
import type { ValueType } from 'recharts/types/component/DefaultTooltipContent';
import StatCard from '../components/StatCard';
import {
  ServerIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ArrowLeftIcon,
  CpuChipIcon,
  MemoryStickIcon,
  HardDiskIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  PencilIcon,
  SignalIcon, // Placeholder for BellIcon
  GlobeAltIcon, // Placeholder for InformationCircleIcon
  ChartBarIcon,
} from '../components/Icons';
import { STATUS_ONLINE, STATUS_OFFLINE, STATUS_REBOOTING, STATUS_PROVISIONING, STATUS_ERROR } from '../types';

// Helper to format date for XAxis
const formatDateTick = (tickItem: string) => {
  const date = new Date(tickItem);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
};

// Helper to format the label in tooltips to local time
const formatTooltipLabel = (label: string) => {
  const date = new Date(label);
  // Using toLocaleString for a more complete date-time representation in the tooltip
  return date.toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
};

// Helper to format percentage for tooltips
const formatPercentForTooltip = (value: ValueType) => {
  if (typeof value === 'number') {
    return `${value.toFixed(2)}%`;
  }
  return `${value}`;
};

// Helper to calculate memory usage percentage
const calculateMemoryUsagePercent = (dataPoint: PerformanceMetricPoint): number | null => {
  if (dataPoint.memoryUsageBytes != null && dataPoint.memoryTotalBytes != null && dataPoint.memoryTotalBytes > 0) {
    return (dataPoint.memoryUsageBytes / dataPoint.memoryTotalBytes) * 100;
  }
  // Check aggregated fields if raw fields are not present or not sufficient
  if (dataPoint.avgMemoryUsageBytes != null && dataPoint.maxMemoryTotalBytes != null && dataPoint.maxMemoryTotalBytes > 0) {
    return (dataPoint.avgMemoryUsageBytes / dataPoint.maxMemoryTotalBytes) * 100;
  }
  return null;
};

// Helper to format Network Speed (Bytes per second)
const formatNetworkSpeed = (bps: number | null | undefined): string => {
  if (bps == null || bps < 0) return 'N/A';
  if (bps === 0) return '0 B/s';
  const k = 1024; // Use 1024 for binary prefixes (KiB, MiB) often used for network speed
  const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s', 'TB/s'];
  // Handle potential log(0) or negative values if bps is very small but positive
  if (bps < 1) return bps.toFixed(2) + ' B/s';
  const i = Math.floor(Math.log(bps) / Math.log(k));
  // Ensure index is within bounds
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
if (bytes < 1) return bytes.toFixed(dm) + ' Bytes'; // Handle very small positive values
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
if (seconds > 0 || uptimeString === '') uptimeString += `${seconds}s`; // Show seconds if other units are zero or it's the only unit

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


const getStatusBadgeClasses = (status: ServerStatus): string => {
  switch (status) {
    case STATUS_ONLINE: return "bg-green-100 text-green-800";
    case STATUS_REBOOTING: return "bg-yellow-100 text-yellow-800 animate-pulse";
    case STATUS_OFFLINE: return "bg-red-100 text-red-800";
    case STATUS_ERROR: return "bg-red-200 text-red-900";
    case STATUS_PROVISIONING: return "bg-blue-100 text-blue-800";
    default: return "bg-slate-100 text-slate-800";
  }
};

const getStatusIcon = (status: ServerStatus): React.ReactNode => {
  switch (status) {
    case STATUS_ONLINE: return <CheckCircleIcon className="w-5 h-5 text-green-500" />;
    case STATUS_REBOOTING: return <ExclamationTriangleIcon className="w-5 h-5 text-yellow-500" />;
    case STATUS_OFFLINE: return <XCircleIcon className="w-5 h-5 text-red-500" />;
    case STATUS_ERROR: return <XCircleIcon className="w-5 h-5 text-red-700" />;
    default: return <ExclamationTriangleIcon className="w-5 h-5 text-slate-500" />;
  }
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
  const [diskIoData, setDiskIoData] = useState<PerformanceMetricPoint[]>([]); // New state for Disk I/O
  const [loadingChartMetrics, setLoadingChartMetrics] = useState(true);
  const [chartError, setChartError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');

  const handleSetSelectedTimeRange = useCallback((value: TimeRangeOption) => {
    setSelectedTimeRange(value);
  }, []);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [isDismissingReminder, setIsDismissingReminder] = useState(false);
  const [dismissReminderError, setDismissReminderError] = useState<string | null>(null);
  const [dismissReminderSuccess, setDismissReminderSuccess] = useState<string | null>(null);

  // State for Service Monitoring
  const [monitorResults, setMonitorResults] = useState<ServiceMonitorResult[]>([]);
  const [loadingMonitors, setLoadingMonitors] = useState(true);
  const [monitorError, setMonitorError] = useState<string | null>(null);

  const handleVpsUpdated = () => {
   // The websocket connection should update the store automatically.
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
     // The websocket update should refresh the vpsDetail.reminderActive state
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
    ? "WebSocket connection error."
    : (connectionStatus === 'connected' && !vpsDetail && !isLoading ? "VPS details not found." : null);

  const timeRangeToMillis: Record<Exclude<TimeRangeOption, 'realtime'>, number> = { '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 };

  useEffect(() => {
    if (!vpsId) {
      setChartError('VPS ID not found.');
      setLoadingChartMetrics(false);
      return;
    }
    // In public view, we don't fetch historical metrics via API for now.
    // We will rely on the `latestMetrics` from the public websocket push.
    if (!isAuthenticated) {
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
          // Calculate interval to get ~300 points
          const intervalSeconds = Math.round(timeRangeToMillis[selectedTimeRange] / 1000 / 300);
          const interval = `${intervalSeconds}s`;
          metrics = await getVpsMetricsTimeseries(vpsId, startTime.toISOString(), endTime.toISOString(), interval);
        }

        if (!isMounted) return;

        const cpuPoints: PerformanceMetricPoint[] = [];
        const memoryPoints: PerformanceMetricPoint[] = [];
        const networkPoints: PerformanceMetricPoint[] = [];
        const diskIoPoints: PerformanceMetricPoint[] = []; // For Disk I/O

        metrics.forEach(point => {
          // For data from /latest-n (realtime initial load), point will have raw fields (e.g., point.cpuUsagePercent).
          // For data from /timeseries (historical), point will have aggregated fields (e.g., point.avgCpuUsagePercent).
          // The chart components expect specific keys (e.g., 'cpuUsagePercent' for CPU chart).
          // We need to ensure the correct value is passed under the expected key.

          // CPU Data: Chart uses 'cpuUsagePercent'
          const cpuValue = point.avgCpuUsagePercent ?? point.cpuUsagePercent;
          if (cpuValue != null) cpuPoints.push({ ...point, cpuUsagePercent: cpuValue });
          
          // Memory Data: Chart uses 'memoryUsagePercent'
          const memoryUsagePercentValue = calculateMemoryUsagePercent(point); // This helper already handles raw vs aggregated
          if (memoryUsagePercentValue != null) memoryPoints.push({ ...point, memoryUsagePercent: memoryUsagePercentValue });
          
          // Network Data: Chart uses 'avgNetworkRxInstantBps' and 'avgNetworkTxInstantBps'
          const rxBps = point.avgNetworkRxInstantBps ?? point.networkRxInstantBps;
          const txBps = point.avgNetworkTxInstantBps ?? point.networkTxInstantBps;
          if (rxBps != null || txBps != null) {
            // Ensure the point pushed to networkData has the keys the chart expects
            networkPoints.push({ ...point, avgNetworkRxInstantBps: rxBps, avgNetworkTxInstantBps: txBps });
          }

          // Disk I/O Data: Chart uses 'avgDiskIoReadBps' and 'avgDiskIoWriteBps'
          const readBps = point.avgDiskIoReadBps ?? point.diskIoReadBps;
          const writeBps = point.avgDiskIoWriteBps ?? point.diskIoWriteBps;
          if (readBps != null || writeBps != null) {
            // Ensure the point pushed to diskIoData has the keys the chart expects
            diskIoPoints.push({ ...point, avgDiskIoReadBps: readBps, avgDiskIoWriteBps: writeBps });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
        setNetworkData(networkPoints);
        setDiskIoData(diskIoPoints); // Set Disk I/O data
      } catch (err) {
        console.error('Failed to fetch chart metrics:', err);
        if (isMounted) setChartError('Could not load chart data.');
      } finally {
        if (isMounted) setLoadingChartMetrics(false);
      }
    };
    fetchChartMetricsData();
    return () => { isMounted = false; };
  }, [vpsId, selectedTimeRange]);

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
          // Fetch last 5 mins of data for "realtime" view initially
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - 300 * 1000);
          results = await getMonitorResultsByVpsId(vpsId, startTime.toISOString(), endTime.toISOString());
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
          results = await getMonitorResultsByVpsId(vpsId, startTime.toISOString(), endTime.toISOString());
        }
        setMonitorResults(results);
      } catch (err) {
        console.error('Failed to fetch service monitor results:', err);
        setMonitorError('Could not load service monitor data.');
      } finally {
        setLoadingMonitors(false);
      }
    };

    fetchMonitorData();
  }, [vpsId, selectedTimeRange, isAuthenticated]);

  // This effect for real-time updates can be simplified or removed if historical fetch is fast enough on range change
  // For now, we keep it to ensure live data continues to flow.
  useEffect(() => {
    // Only apply websocket updates in real-time mode
    if (selectedTimeRange !== 'realtime' || !vpsDetail?.latestMetrics || !vpsId || !isAuthenticated) {
      return;
    }

    const newMetrics = vpsDetail.latestMetrics;
    const newTime = newMetrics.time;
    const numericVpsId = parseInt(vpsId, 10);

    const appendAndTrim = <T extends { time: string }>(prevData: T[], newDataPoint: T): T[] => {
      // Check for duplicates based on timestamp
      if (prevData.some(p => p.time === newDataPoint.time)) {
        return prevData;
      }
      // Ensure data is sorted by time
      const combined = [...prevData, newDataPoint].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
      return combined.length > 300 ? combined.slice(combined.length - 300) : combined;
    };

    // newMetrics is of type LatestPerformanceMetric (all camelCase)
    if (newMetrics.cpuUsagePercent != null) {
      setCpuData(prev => appendAndTrim(prev, {
        time: newTime, vpsId: numericVpsId,
        cpuUsagePercent: newMetrics.cpuUsagePercent, // Chart expects cpuUsagePercent
        // Fill other PerformanceMetricPoint fields as null/undefined or with defaults
        avgCpuUsagePercent: null, avgMemoryUsageBytes: null, maxMemoryTotalBytes: null,
        memoryUsageBytes: null, memoryTotalBytes: null, memoryUsagePercent: null,
        avgNetworkRxInstantBps: null, avgNetworkTxInstantBps: null, networkRxInstantBps: null, networkTxInstantBps: null,
        avgDiskIoReadBps: null, avgDiskIoWriteBps: null, diskIoReadBps: null, diskIoWriteBps: null,
        swapUsageBytes: null, swapTotalBytes: null,
      }));
    }
    
    // Create a temporary PerformanceMetricPoint compatible object for calculateMemoryUsagePercent
    const tempMemoryPoint: Partial<PerformanceMetricPoint> = {
        memoryUsageBytes: newMetrics.memoryUsageBytes,
        memoryTotalBytes: newMetrics.memoryTotalBytes,
    };
    const memoryUsagePercentValue = calculateMemoryUsagePercent(tempMemoryPoint as PerformanceMetricPoint);

    if (memoryUsagePercentValue != null) {
      setMemoryData(prev => appendAndTrim(prev, {
        time: newTime, vpsId: numericVpsId,
        memoryUsagePercent: memoryUsagePercentValue, // Chart expects memoryUsagePercent
        memoryUsageBytes: newMetrics.memoryUsageBytes,
        memoryTotalBytes: newMetrics.memoryTotalBytes,
        // Fill other PerformanceMetricPoint fields as null/undefined
        cpuUsagePercent: null, avgCpuUsagePercent: null, avgMemoryUsageBytes: null, maxMemoryTotalBytes: null,
        avgNetworkRxInstantBps: null, avgNetworkTxInstantBps: null, networkRxInstantBps: null, networkTxInstantBps: null,
        avgDiskIoReadBps: null, avgDiskIoWriteBps: null, diskIoReadBps: null, diskIoWriteBps: null,
        swapUsageBytes: null, swapTotalBytes: null,
      }));
    }

    if (newMetrics.networkRxInstantBps != null || newMetrics.networkTxInstantBps != null) {
      setNetworkData(prev => appendAndTrim(prev, {
        time: newTime, vpsId: numericVpsId,
        avgNetworkRxInstantBps: newMetrics.networkRxInstantBps, // Chart expects avgNetworkRxInstantBps
        avgNetworkTxInstantBps: newMetrics.networkTxInstantBps, // Chart expects avgNetworkTxInstantBps
        networkRxInstantBps: newMetrics.networkRxInstantBps, // Keep raw value
        networkTxInstantBps: newMetrics.networkTxInstantBps, // Keep raw value
        // Fill other PerformanceMetricPoint fields as null/undefined
        cpuUsagePercent: null, avgCpuUsagePercent: null, avgMemoryUsageBytes: null, maxMemoryTotalBytes: null,
        memoryUsageBytes: null, memoryTotalBytes: null, memoryUsagePercent: null,
        avgDiskIoReadBps: null, avgDiskIoWriteBps: null, diskIoReadBps: null, diskIoWriteBps: null,
        swapUsageBytes: null, swapTotalBytes: null,
      }));
    }

    if (newMetrics.diskIoReadBps != null || newMetrics.diskIoWriteBps != null) {
      setDiskIoData(prev => appendAndTrim(prev, {
        time: newTime, vpsId: numericVpsId,
        avgDiskIoReadBps: newMetrics.diskIoReadBps,    // Chart expects avgDiskIoReadBps
        avgDiskIoWriteBps: newMetrics.diskIoWriteBps,  // Chart expects avgDiskIoWriteBps
        diskIoReadBps: newMetrics.diskIoReadBps,       // Keep raw value
        diskIoWriteBps: newMetrics.diskIoWriteBps,     // Keep raw value
        // Fill other PerformanceMetricPoint fields as null/undefined
        cpuUsagePercent: null, avgCpuUsagePercent: null, avgMemoryUsageBytes: null, maxMemoryTotalBytes: null,
        memoryUsageBytes: null, memoryTotalBytes: null, memoryUsagePercent: null,
        avgNetworkRxInstantBps: null, avgNetworkTxInstantBps: null, networkRxInstantBps: null, networkTxInstantBps: null,
        swapUsageBytes: null, swapTotalBytes: null,
      }));
    }
  }, [vpsDetail?.latestMetrics?.time, vpsId, selectedTimeRange, isAuthenticated]);

  // WebSocket listener for new monitor results
  useEffect(() => {
    if (selectedTimeRange !== 'realtime' || !vpsId || !isAuthenticated) {
        return;
    }

    const handleNewMonitorResult = (result: ServiceMonitorResult) => {
        // Check if the result belongs to any of the monitors associated with this VPS.
        // This requires knowing which monitors are on this VPS.
        // For now, we'll accept any result and let the chart component filter.
        // A better approach would be to have a list of monitor IDs for this VPS.
        setMonitorResults(prevResults => {
            const updatedResults = [...prevResults, result].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
            // Keep a rolling window of data, e.g., last 300 points
            return updatedResults.length > 300 ? updatedResults.slice(updatedResults.length - 300) : updatedResults;
        });
    };

    websocketService.on('service_monitor_result', handleNewMonitorResult);

    return () => {
        websocketService.off('service_monitor_result', handleNewMonitorResult);
    };
}, [vpsId, selectedTimeRange, isAuthenticated]);


  if (isLoadingPage) {
    return <div className="flex justify-center items-center h-64"><p>Loading server details...</p></div>;
  }

  if (pageError) {
    return (
      <div className="text-center py-10">
        <XCircleIcon className="w-16 h-16 text-red-500 mx-auto mb-4" />
        <p className="text-xl text-red-600 bg-red-100 p-4 rounded-lg">{pageError}</p>
        <Link to={isAuthenticated ? "/servers" : "/"} className="mt-6 inline-flex items-center bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors">
          <ArrowLeftIcon className="w-5 h-5 inline mr-2" />Back to Dashboard
        </Link>
      </div>
    );
  }

  if (!vpsDetail) {
    return <div className="text-center text-slate-500">Server data not available.</div>;
  }

  const { latestMetrics: metrics } = vpsDetail;
  const { metadata } = vpsDetail;
  const memUsed = metrics?.memoryUsageBytes ?? 0;
  const memTotal = metrics?.memoryTotalBytes ?? 0;
  const diskUsed = metrics?.diskUsedBytes ?? 0;
  const diskTotal = metrics?.diskTotalBytes ?? 0;

  // Traffic data calculation
  const { trafficLimitBytes: trafficLimit, trafficCurrentCycleRxBytes, trafficCurrentCycleTxBytes, trafficBillingRule: billingRule } = vpsDetail;
  const currentRx = trafficCurrentCycleRxBytes ?? 0;
  const currentTx = trafficCurrentCycleTxBytes ?? 0;

  let totalUsedTraffic = 0;
  if (billingRule === 'sum_in_out') {
    totalUsedTraffic = currentRx + currentTx;
  } else if (billingRule === 'out_only') {
    totalUsedTraffic = currentTx;
  } else if (billingRule === 'max_in_out') {
    totalUsedTraffic = Math.max(currentRx, currentTx);
  } else if (billingRule) { // If rule is set but unknown, sum by default or show error? For now, sum.
    totalUsedTraffic = currentRx + currentTx;
  }


  const trafficRemaining = trafficLimit != null ? trafficLimit - totalUsedTraffic : null;
  const trafficUsagePercent = trafficLimit != null && trafficLimit > 0 ? (totalUsedTraffic / trafficLimit) * 100 : null;

  return (
    <div className="p-4 md:p-6 lg:p-8 space-y-8 bg-slate-50 min-h-screen">
      {/* Header Section */}
      <section className="bg-white p-6 rounded-xl shadow-md">
        <div className="flex flex-col sm:flex-row justify-between items-start">
          <div>
            <div className="flex items-center mb-2">
              <ServerIcon className="w-8 h-8 mr-3 text-indigo-600 flex-shrink-0" />
              <h1 className="text-3xl font-bold text-slate-800">{vpsDetail.name}</h1>
            </div>
            {vpsDetail.ipAddress && (
              <p className="text-slate-500 mt-1 ml-11">IP: {vpsDetail.ipAddress}</p>
            )}
          </div>
          <div className="mt-4 sm:mt-0 sm:text-right space-y-2">
            <div className={`px-3 py-1.5 text-sm font-semibold rounded-full inline-flex items-center gap-2 ${getStatusBadgeClasses(vpsDetail.status)}`}>
              {getStatusIcon(vpsDetail.status)}
              {vpsDetail.status.toUpperCase()}
            </div>
            <div className="flex items-center space-x-2">
              {isAuthenticated && (
                <button
                    onClick={() => setIsEditModalOpen(true)}
                    className="inline-flex items-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm"
                    aria-label={`Edit ${vpsDetail.name}`}
                >
                    <PencilIcon className="w-4 h-4 mr-1.5" /> 编辑
                </button>
              )}
               <Link to={isAuthenticated ? "/servers" : "/"} className="inline-flex items-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm">
                   <ArrowLeftIcon className="w-4 h-4 mr-1.5" /> Dashboard
               </Link>
            </div>
          </div>
        </div>
      </section>
      {isAuthenticated && (
        <EditVpsModal
          isOpen={isEditModalOpen}
          onClose={() => setIsEditModalOpen(false)}
          vps={vpsDetail}
          allVps={servers}
          onVpsUpdated={handleVpsUpdated}
        />
      )}

      {/* Quick Stats Section */}
      <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
        <StatCard title="CPU Usage" value={metrics?.cpuUsagePercent?.toFixed(1) ?? 'N/A'} unit="%" icon={<CpuChipIcon />} colorClass="text-blue-500" description={vpsDetail.status === 'offline' ? "Offline" : `${metadata?.cpu_static_info?.brand || ''}`} />
        <StatCard title="RAM Usage" value={memTotal > 0 ? ((memUsed / memTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<MemoryStickIcon />} colorClass="text-purple-500" description={vpsDetail.status === 'offline' ? "Offline" : `${formatBytes(memUsed)} / ${formatBytes(memTotal)}`} />
        <StatCard title="Disk Usage" value={diskTotal > 0 ? ((diskUsed / diskTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<HardDiskIcon />} colorClass="text-orange-500" description={vpsDetail.status === 'offline' ? "Offline" : `${formatBytes(diskUsed)} / ${formatBytes(diskTotal)}`} />
        <StatCard title="Upload" value={formatNetworkSpeed(metrics?.networkTxInstantBps)} icon={<ArrowUpIcon />} colorClass="text-green-500" description="Current outgoing" />
        <StatCard title="Download" value={formatNetworkSpeed(metrics?.networkRxInstantBps)} icon={<ArrowDownIcon />} colorClass="text-sky-500" description="Current incoming" />
        <StatCard title="Uptime" value={formatUptime(metrics?.uptimeSeconds)} icon={<ExclamationTriangleIcon />} colorClass="text-teal-500" description="Current session" />
      </section>

      {/* Traffic Information Section */}
      {isAuthenticated && vpsDetail.trafficBillingRule && (
        <section className="bg-white p-6 rounded-xl shadow-md">
          <h2 className="text-xl font-semibold text-slate-700 mb-6">流量信息</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
            <InfoBlock title="流量限额" value={trafficLimit != null ? formatBytes(trafficLimit) : '未设置'} />
            <InfoBlock title="计费规则" value={formatTrafficBillingRule(billingRule)} />
            <InfoBlock title="下次重置时间" value={formatTrafficDate(vpsDetail.nextTrafficResetAt)} />
            
            <InfoBlock title="本周期已用 (总计)" value={formatBytes(totalUsedTraffic)} />
            {trafficLimit != null && (
              <InfoBlock
                title="本周期剩余"
                value={trafficRemaining != null ? formatBytes(trafficRemaining) : 'N/A'}
              />
            )}
            {trafficUsagePercent != null && (
              <InfoBlock
                title="使用率"
                value={`${trafficUsagePercent.toFixed(2)}%`}
              />
            )}
            <InfoBlock title="本周期 RX (入站)" value={formatBytes(currentRx)} />
            <InfoBlock title="本周期 TX (出站)" value={formatBytes(currentTx)} />
            <InfoBlock title="上次重置时间" value={formatTrafficDate(vpsDetail.trafficLastResetAt)} />
          </div>
        </section>
      )}

      {/* Charts Section */}
      <section className="bg-white p-4 rounded-xl shadow-md">
        <div className="flex flex-col sm:flex-row justify-between items-center mb-4">
          <h2 className="text-xl font-semibold text-slate-700">Performance Metrics</h2>
          <div className="flex items-center space-x-1 mt-3 sm:mt-0 p-1 bg-slate-100 rounded-lg">
            {TIME_RANGE_OPTIONS.map(period => (
              <button
                key={period.value}
                onClick={() => handleSetSelectedTimeRange(period.value)}
                aria-pressed={selectedTimeRange === period.value}
                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${selectedTimeRange === period.value ? 'bg-indigo-600 text-white shadow' : 'text-slate-600 hover:bg-slate-200'}`}
              >
                {period.label}
              </button>
            ))}
          </div>
        </div>
        {chartError && <p className="text-red-500 text-center">{chartError}</p>}
        {loadingChartMetrics ? <div className="h-72 flex justify-center items-center"><p>Loading charts...</p></div> : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
            <ChartComponent title="CPU Usage (%)" data={cpuData} dataKey="cpuUsagePercent" stroke="#8884d8" yDomain={[0, 100]} />
            <ChartComponent title="Memory Usage (%)" data={memoryData} dataKey="memoryUsagePercent" stroke="#82ca9d" yDomain={[0, 100]} />
            {/* Network and Disk I/O charts will now be side-by-side on lg screens */}
            <NetworkChartComponent data={networkData} /> {/* Uses avgNetworkRxInstantBps, avgNetworkTxInstantBps internally */}
            <DiskIoChartComponent data={diskIoData} />   {/* Uses avgDiskIoReadBps, avgDiskIoWriteBps internally */}
          </div>
        )}
      </section>

      {/* Service Monitoring Section */}
      {isAuthenticated && (
        <section className="bg-white p-4 rounded-xl shadow-md">
            <div className="flex flex-col sm:flex-row justify-between items-center mb-4">
                <h2 className="text-xl font-semibold text-slate-700 flex items-center">
                    <ChartBarIcon className="w-6 h-6 mr-2 text-indigo-500" />
                    Service Monitoring
                </h2>
                {/* Time range selector is shared with performance metrics */}
            </div>
            {monitorError && <p className="text-red-500 text-center">{monitorError}</p>}
            {loadingMonitors ? <div className="h-72 flex justify-center items-center"><p>Loading monitor charts...</p></div> : (
                <ServiceMonitorChartComponent results={monitorResults} />
            )}
        </section>
      )}

      {/* System Info Section */}
      <section className="bg-white p-6 rounded-xl shadow-md">
        <h2 className="text-xl font-semibold text-slate-700 mb-6">System Information</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-6 text-sm">
          <InfoBlock title="Hostname" value={`${metadata?.hostname || 'N/A'}`} />
          <InfoBlock title="Operating System" value={`${metadata?.os_name || 'N/A'} (${metadata?.long_os_version || metadata?.os_version_detail || 'N/A'})`} />
          <InfoBlock title="Distribution ID" value={`${metadata?.distribution_id || 'N/A'}`} />
          <InfoBlock title="Kernel Version" value={`${metadata?.kernel_version || 'N/A'}`} />
          <InfoBlock title="Architecture" value={`${metadata?.arch || 'N/A'}`} />
          <InfoBlock title="CPU Brand" value={`${metadata?.cpu_static_info?.brand || 'N/A'}`} />
          <InfoBlock title="CPU Name" value={`${metadata?.cpu_static_info?.name || 'N/A'}`} />
          <InfoBlock title="CPU Frequency" value={metadata?.cpu_static_info?.frequency ? `${metadata.cpu_static_info.frequency} MHz` : 'N/A'} />
          <InfoBlock title="CPU Vendor ID" value={`${metadata?.cpu_static_info?.vendorId || 'N/A'}`} />
          <InfoBlock title="Physical Cores" value={`${metadata?.physical_core_count ?? 'N/A'}`} />
          <InfoBlock title="Total RAM" value={formatBytes(metadata?.total_memory_bytes ?? metrics?.memoryTotalBytes)} />
          <InfoBlock title="Total Swap" value={formatBytes(metadata?.total_swap_bytes)} />
          <InfoBlock title="Total Disk" value={formatBytes(metrics?.diskTotalBytes)} /> {/* Disk info usually comes from metrics */}
        </div>
      </section>

      {/* Renewal Information Section */}
      {isAuthenticated && (vpsDetail.renewalCycle || vpsDetail.nextRenewalDate || vpsDetail.paymentMethod) && (
        <section className="bg-white p-6 rounded-xl shadow-md">
          <div className="flex justify-between items-center mb-6">
            <h2 className="text-xl font-semibold text-slate-700 flex items-center">
              <GlobeAltIcon className="w-6 h-6 mr-2 text-indigo-500" /> {/* Placeholder Icon */}
              续费信息
            </h2>
            {vpsDetail.reminderActive && (
              <button
                onClick={handleDismissReminder}
                disabled={isDismissingReminder}
                className="inline-flex items-center bg-yellow-100 hover:bg-yellow-200 text-yellow-800 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <SignalIcon className="w-4 h-4 mr-1.5" /> {/* Placeholder Icon */}
                {isDismissingReminder ? '清除中...' : '清除提醒'}
              </button>
            )}
          </div>

          {dismissReminderError && <p className="text-sm text-red-600 bg-red-100 p-2 rounded-md mb-4">{dismissReminderError}</p>}
          {dismissReminderSuccess && <p className="text-sm text-green-600 bg-green-100 p-2 rounded-md mb-4">{dismissReminderSuccess}</p>}
          
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
        </section>
      )}
    </div>
  );
};

const ChartComponent: React.FC<{ title: string, data: PerformanceMetricPoint[], dataKey: string, stroke: string, yDomain: [number, number] }> = React.memo(({ title, data, dataKey, stroke, yDomain }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-slate-600 text-center mb-2 flex-shrink-0">{title}</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: -10, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }}  />
            <YAxis domain={yDomain} tick={{ fontSize: 11 }} tickFormatter={(tick) => `${tick}%`} />
            <Tooltip formatter={formatPercentForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey={dataKey} stroke={stroke} dot={false} name={title.split(' ')[0]} isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-slate-500 pt-16">No data available.</p>}
  </div>
));

const NetworkChartComponent: React.FC<{ data: PerformanceMetricPoint[] }> = React.memo(({ data }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-slate-600 text-center mb-2 flex-shrink-0">Network Speed</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
            <YAxis tickFormatter={formatNetworkSpeed} width={80} tick={{ fontSize: 11 }} />
            <Tooltip formatter={(value: ValueType) => formatNetworkSpeed(value as number)} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey="avgNetworkRxInstantBps" stroke="#38bdf8" dot={false} name="Download" isAnimationActive={false} />
            <Line type="monotone" dataKey="avgNetworkTxInstantBps" stroke="#34d399" dot={false} name="Upload" isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-slate-500 pt-16">No data available.</p>}
  </div>
));

const DiskIoChartComponent: React.FC<{ data: PerformanceMetricPoint[] }> = React.memo(({ data }) => (
  <div className="h-72 flex flex-col">
    <h3 className="text-lg font-semibold text-slate-600 text-center mb-2 flex-shrink-0">Disk I/O Speed</h3>
    {data.length > 0 ? (
      <div className="flex-grow">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
            <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }}  />
            <YAxis tickFormatter={formatNetworkSpeed} width={80} tick={{ fontSize: 11 }} /> {/* Re-use formatNetworkSpeed for BPS */}
            <Tooltip formatter={(value: ValueType) => formatNetworkSpeed(value as number)} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
            <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
            <Line type="monotone" dataKey="avgDiskIoReadBps" stroke="#ff7300" dot={false} name="Read" isAnimationActive={false} /> {/* Orange for Read */}
            <Line type="monotone" dataKey="avgDiskIoWriteBps" stroke="#387908" dot={false} name="Write" isAnimationActive={false} /> {/* Dark Green for Write */}
          </LineChart>
        </ResponsiveContainer>
      </div>
    ) : <p className="text-center text-slate-500 pt-16">No data available.</p>}
  </div>
));

// Helper to format latency for tooltips
const formatLatencyForTooltip = (value: ValueType) => {
  if (typeof value === 'number') {
    return `${value.toFixed(0)} ms`;
  }
  return `${value}`;
};

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];

const ServiceMonitorChartComponent: React.FC<{ results: ServiceMonitorResult[] }> = React.memo(({ results }) => {
    const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});

    const { chartData, monitorLines, downtimeAreas } = useMemo(() => {
        if (!results || results.length === 0) {
            return { chartData: [], monitorLines: [], downtimeAreas: [] };
        }

        // Sort all results by time initially
        const sortedResults = [...results].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());

        // Group results by monitor ID
        const groupedByMonitorId = sortedResults.reduce((acc, result) => {
            const monitorId = result.monitorId;
            if (!acc[monitorId]) {
                acc[monitorId] = [];
            }
            acc[monitorId].push(result);
            return acc;
        }, {} as Record<number, ServiceMonitorResult[]>);

        // Create line definitions and assign colors
        const monitorLines: { dataKey: string; name: string; stroke: string }[] = [];
        let colorIndex = 0;
        for (const monitorId in groupedByMonitorId) {
            if (Object.prototype.hasOwnProperty.call(groupedByMonitorId, monitorId)) {
                const firstResult = groupedByMonitorId[monitorId][0];
                const monitorName = firstResult.monitorName || `Monitor #${monitorId}`;
                const color = AGENT_COLORS[colorIndex % AGENT_COLORS.length];
                
                monitorLines.push({
                    dataKey: `monitor_${monitorId}`,
                    name: monitorName,
                    stroke: color,
                });
                colorIndex++;
            }
        }

        // Get all unique time points across all monitors
        const timePoints = [...new Set(sortedResults.map(r => new Date(r.time).toISOString()))].sort();

        // Create the chart data by pivoting the results
        const chartData = timePoints.map(time => {
            const point: { time: string; [key: string]: number | null | string } = { time };
            for (const monitorId in groupedByMonitorId) {
                const dataKey = `monitor_${monitorId}`;
                const resultForTime = groupedByMonitorId[monitorId].find(r => new Date(r.time).toISOString() === time);
                point[dataKey] = resultForTime && resultForTime.isUp ? resultForTime.latencyMs : null;
            }
            return point;
        });

        // Calculate downtime areas
        const areas: { x1: string, x2: string }[] = [];
        let downtimeStart: string | null = null;

        for (let i = 0; i < timePoints.length; i++) {
            const time = timePoints[i];
            const isAnyDown = Object.values(groupedByMonitorId).some(monitorResults =>
                monitorResults.some(r => new Date(r.time).toISOString() === time && !r.isUp)
            );

            if (isAnyDown && !downtimeStart) {
                downtimeStart = time;
            } else if (!isAnyDown && downtimeStart) {
                // Find the previous time point to end the area
                const prevTime = i > 0 ? timePoints[i-1] : downtimeStart;
                areas.push({ x1: downtimeStart, x2: prevTime });
                downtimeStart = null;
            }
        }
        // If downtime continues to the end
        if (downtimeStart) {
            areas.push({ x1: downtimeStart, x2: timePoints[timePoints.length - 1] });
        }

        return { chartData, monitorLines, downtimeAreas: areas };
    }, [results]);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const handleLegendClick = (data: any) => {
        const dataKey = data.dataKey;
        if (typeof dataKey === 'string') {
            setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
        }
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const renderLegendText = (value: string, entry: any) => {
        const { color, dataKey } = entry;
        const isHidden = typeof dataKey === 'string' && hiddenLines[dataKey];
        return <span style={{ color: isHidden ? '#A0A0A0' : color || '#000', cursor: 'pointer' }}>{value}</span>;
    };

    if (results.length === 0) {
        return <p className="text-center text-slate-500 pt-16">No service monitoring data available for this VPS.</p>;
    }

    return (
        <div className="h-80">
            <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                    <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
                    <YAxis tickFormatter={(tick) => `${tick} ms`} width={80} tick={{ fontSize: 11 }} />
                    <Tooltip formatter={formatLatencyForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
                    <Legend wrapperStyle={{ fontSize: '0.8rem' }} onClick={handleLegendClick} formatter={renderLegendText} />
                    {downtimeAreas.map((area, index) => (
                        <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="red" fillOpacity={0.15} ifOverflow="visible" />
                    ))}
                    {monitorLines.map((line: { dataKey: string; name: string; stroke: string }) => (
                        <Line
                            key={line.dataKey}
                            type="monotone"
                            dataKey={line.dataKey}
                            name={line.name}
                            stroke={hiddenLines[line.dataKey] ? 'transparent' : line.stroke}
                            dot={false}
                            connectNulls={true}
                            strokeWidth={2}
                            isAnimationActive={false}
                        />
                    ))}
                </LineChart>
            </ResponsiveContainer>
        </div>
    );
});

const InfoBlock: React.FC<{ title: string, value: string }> = React.memo(({ title, value }) => (
  <div className="space-y-1">
    <p className="font-medium text-slate-600 block">{title}</p>
    <p className="text-slate-800">{value}</p>
  </div>
));

export default VpsDetailPage;