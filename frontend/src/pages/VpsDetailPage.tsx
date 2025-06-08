import React, { useEffect, useState, useMemo } from 'react';
import { useParams, Link } from 'react-router-dom';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { getVpsMetricsTimeseries, getLatestNMetrics } from '../services/metricsService';
import type { PerformanceMetricPoint, ServerStatus } from '../types';
import { useServerListStore } from '../store/serverListStore';
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
  if (dataPoint.memory_usage_bytes != null && dataPoint.memory_total_bytes != null && dataPoint.memory_total_bytes > 0) {
    return (dataPoint.memory_usage_bytes / dataPoint.memory_total_bytes) * 100;
  }
  // Check aggregated fields if raw fields are not present or not sufficient
  if (dataPoint.avg_memory_usage_bytes != null && dataPoint.max_memory_total_bytes != null && dataPoint.max_memory_total_bytes > 0) {
    return (dataPoint.avg_memory_usage_bytes / dataPoint.max_memory_total_bytes) * 100;
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

  const { servers, connectionStatus, isLoading: isServerListLoading } = useServerListStore(useShallow(state => ({
    servers: state.servers,
    connectionStatus: state.connectionStatus,
    isLoading: state.isLoading,
  })));

  const vpsDetail = useMemo(() => {
    if (!vpsId) return null;
    const numericVpsId = parseInt(vpsId, 10);
    return servers.find(server => server.id === numericVpsId) || null;
  }, [servers, vpsId]);

  const [cpuData, setCpuData] = useState<PerformanceMetricPoint[]>([]);
  const [memoryData, setMemoryData] = useState<PerformanceMetricPoint[]>([]);
  const [networkData, setNetworkData] = useState<PerformanceMetricPoint[]>([]);
  const [loadingChartMetrics, setLoadingChartMetrics] = useState(true);
  const [chartError, setChartError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);

  const handleVpsUpdated = () => {
   // The websocket connection should update the store automatically.
   console.log('VPS updated, store should refresh via WebSocket.');
   setIsEditModalOpen(false);
  };

  const isLoadingPage = isServerListLoading && !vpsDetail;
  const pageError = connectionStatus === 'error' || connectionStatus === 'permanently_failed'
    ? "WebSocket connection error."
    : (connectionStatus === 'connected' && !vpsDetail && !isServerListLoading ? "VPS details not found." : null);

  const timeRangeToMillis: Record<Exclude<TimeRangeOption, 'realtime'>, number> = { '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 };

  useEffect(() => {
    if (!vpsId) {
      setChartError('VPS ID not found.');
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

        metrics.forEach(point => {
          const cpuValue = point.avg_cpu_usage_percent ?? point.cpu_usage_percent;
          if (cpuValue != null) cpuPoints.push({ ...point, cpu_usage_percent: cpuValue });
          const memoryUsagePercent = calculateMemoryUsagePercent(point);
          if (memoryUsagePercent != null) memoryPoints.push({ ...point, memory_usage_percent: memoryUsagePercent });
          
          const rxBps = point.avg_network_rx_instant_bps ?? point.network_rx_instant_bps;
          const txBps = point.avg_network_tx_instant_bps ?? point.network_tx_instant_bps;
          if (rxBps != null || txBps != null) {
            networkPoints.push({ ...point, avg_network_rx_instant_bps: rxBps, avg_network_tx_instant_bps: txBps });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
        setNetworkData(networkPoints);
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

  // This effect for real-time updates can be simplified or removed if historical fetch is fast enough on range change
  // For now, we keep it to ensure live data continues to flow.
  useEffect(() => {
    // Only apply websocket updates in real-time mode
    if (selectedTimeRange !== 'realtime' || !vpsDetail?.latestMetrics || !vpsId) {
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

    if (newMetrics.cpuUsagePercent != null) {
      setCpuData(prev => appendAndTrim(prev, { time: newTime, vps_id: numericVpsId, cpu_usage_percent: newMetrics.cpuUsagePercent }));
    }
    const memoryUsagePercent = calculateMemoryUsagePercent({ memory_usage_bytes: newMetrics.memoryUsageBytes, memory_total_bytes: newMetrics.memoryTotalBytes } as PerformanceMetricPoint);
    if (memoryUsagePercent != null) {
      setMemoryData(prev => appendAndTrim(prev, { time: newTime, vps_id: numericVpsId, memory_usage_percent: memoryUsagePercent }));
    }
    if (newMetrics.networkRxInstantBps != null || newMetrics.networkTxInstantBps != null) {
      setNetworkData(prev => appendAndTrim(prev, { time: newTime, vps_id: numericVpsId, avg_network_rx_instant_bps: newMetrics.networkRxInstantBps, avg_network_tx_instant_bps: newMetrics.networkTxInstantBps }));
    }
  }, [vpsDetail?.latestMetrics?.time, vpsId, selectedTimeRange]);


  if (isLoadingPage) {
    return <div className="flex justify-center items-center h-64"><p>Loading server details...</p></div>;
  }

  if (pageError) {
    return (
      <div className="text-center py-10">
        <XCircleIcon className="w-16 h-16 text-red-500 mx-auto mb-4" />
        <p className="text-xl text-red-600 bg-red-100 p-4 rounded-lg">{pageError}</p>
        <Link to="/" className="mt-6 inline-flex items-center bg-indigo-600 hover:bg-indigo-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors">
          <ArrowLeftIcon className="w-5 h-5 inline mr-2" />Back to Dashboard
        </Link>
      </div>
    );
  }

  if (!vpsDetail) {
    return <div className="text-center text-slate-500">Server data not available.</div>;
  }

  const { latestMetrics: metrics, metadata } = vpsDetail;
  const memUsed = metrics?.memoryUsageBytes ?? 0;
  const memTotal = metrics?.memoryTotalBytes ?? 0;
  const diskUsed = metrics?.diskUsedBytes ?? 0;
  const diskTotal = metrics?.diskTotalBytes ?? 0;

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
            <p className="text-slate-500 mt-1 ml-11">IP: {vpsDetail.ipAddress || 'N/A'}</p>
          </div>
          <div className="mt-4 sm:mt-0 sm:text-right space-y-2">
            <div className={`px-3 py-1.5 text-sm font-semibold rounded-full inline-flex items-center gap-2 ${getStatusBadgeClasses(vpsDetail.status)}`}>
              {getStatusIcon(vpsDetail.status)}
              {vpsDetail.status.toUpperCase()}
            </div>
           <div className="flex items-center space-x-2">
               <button
                   onClick={() => setIsEditModalOpen(true)}
                   className="inline-flex items-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm"
                   aria-label={`Edit ${vpsDetail.name}`}
               >
                   <PencilIcon className="w-4 h-4 mr-1.5" /> 编辑
               </button>
               <Link to="/" className="inline-flex items-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm">
                   <ArrowLeftIcon className="w-4 h-4 mr-1.5" /> Dashboard
               </Link>
           </div>
          </div>
        </div>
      </section>
     <EditVpsModal
       isOpen={isEditModalOpen}
       onClose={() => setIsEditModalOpen(false)}
       vps={vpsDetail}
       allVps={servers}
       onVpsUpdated={handleVpsUpdated}
     />

      {/* Quick Stats Section */}
      <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
        <StatCard title="CPU Usage" value={metrics?.cpuUsagePercent?.toFixed(1) ?? 'N/A'} unit="%" icon={<CpuChipIcon />} colorClass="text-blue-500" description={vpsDetail.status === 'offline' ? "Offline" : `${metadata?.cpu_static_info?.brand || ''}`} />
        <StatCard title="RAM Usage" value={memTotal > 0 ? ((memUsed / memTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<MemoryStickIcon />} colorClass="text-purple-500" description={vpsDetail.status === 'offline' ? "Offline" : `${formatBytes(memUsed)} / ${formatBytes(memTotal)}`} />
        <StatCard title="Disk Usage" value={diskTotal > 0 ? ((diskUsed / diskTotal) * 100).toFixed(1) : 'N/A'} unit="%" icon={<HardDiskIcon />} colorClass="text-orange-500" description={vpsDetail.status === 'offline' ? "Offline" : `${formatBytes(diskUsed)} / ${formatBytes(diskTotal)}`} />
        <StatCard title="Upload" value={formatNetworkSpeed(metrics?.networkTxInstantBps)} icon={<ArrowUpIcon />} colorClass="text-green-500" description="Current outgoing" />
        <StatCard title="Download" value={formatNetworkSpeed(metrics?.networkRxInstantBps)} icon={<ArrowDownIcon />} colorClass="text-sky-500" description="Current incoming" />
        <StatCard title="Uptime" value={formatUptime(metrics?.uptimeSeconds)} icon={<ExclamationTriangleIcon />} colorClass="text-teal-500" description="Current session" />
      </section>

      {/* Charts Section */}
      <section className="bg-white p-4 rounded-xl shadow-md">
        <div className="flex flex-col sm:flex-row justify-between items-center mb-4">
          <h2 className="text-xl font-semibold text-slate-700">Performance Metrics</h2>
          <div className="flex items-center space-x-1 mt-3 sm:mt-0 p-1 bg-slate-100 rounded-lg">
            {TIME_RANGE_OPTIONS.map(period => (
              <button
                key={period.value}
                onClick={() => setSelectedTimeRange(period.value)}
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
            <ChartComponent title="CPU Usage (%)" data={cpuData} dataKey="cpu_usage_percent" stroke="#8884d8" yDomain={[0, 100]} />
            <ChartComponent title="Memory Usage (%)" data={memoryData} dataKey="memory_usage_percent" stroke="#82ca9d" yDomain={[0, 100]} />
            <div className="lg:col-span-2">
              <NetworkChartComponent data={networkData} />
            </div>
          </div>
        )}
      </section>

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
    </div>
  );
};

const ChartComponent: React.FC<{ title: string, data: PerformanceMetricPoint[], dataKey: string, stroke: string, yDomain: [number, number] }> = ({ title, data, dataKey, stroke, yDomain }) => (
  <div className="h-72">
    <h3 className="text-lg font-semibold text-slate-600 text-center mb-2">{title}</h3>
    {data.length > 0 ? (
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 5, right: 20, left: -10, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
          <YAxis domain={yDomain} tick={{ fontSize: 11 }} tickFormatter={(tick) => `${tick}%`} />
          <Tooltip formatter={formatPercentForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
          <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
          <Line type="monotone" dataKey={dataKey} stroke={stroke} dot={false} name={title.split(' ')[0]} />
        </LineChart>
      </ResponsiveContainer>
    ) : <p className="text-center text-slate-500 pt-16">No data available.</p>}
  </div>
);

const NetworkChartComponent: React.FC<{ data: PerformanceMetricPoint[] }> = ({ data }) => (
  <div className="h-72">
    <h3 className="text-lg font-semibold text-slate-600 text-center mb-2">Network Speed</h3>
    {data.length > 0 ? (
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
          <YAxis tickFormatter={formatNetworkSpeed} width={80} tick={{ fontSize: 11 }} />
          <Tooltip formatter={(value: ValueType) => formatNetworkSpeed(value as number)} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.8)', backdropFilter: 'blur(2px)', borderRadius: '0.5rem', fontSize: '0.8rem' }} />
          <Legend wrapperStyle={{ fontSize: '0.8rem' }} />
          <Line type="monotone" dataKey="avg_network_rx_instant_bps" stroke="#38bdf8" dot={false} name="Download" />
          <Line type="monotone" dataKey="avg_network_tx_instant_bps" stroke="#34d399" dot={false} name="Upload" />
        </LineChart>
      </ResponsiveContainer>
    ) : <p className="text-center text-slate-500 pt-16">No data available.</p>}
  </div>
);

const InfoBlock: React.FC<{ title: string, value: string }> = ({ title, value }) => (
  <div className="space-y-1">
    <p className="font-medium text-slate-600 block">{title}</p>
    <p className="text-slate-800">{value}</p>
  </div>
);

export default VpsDetailPage;