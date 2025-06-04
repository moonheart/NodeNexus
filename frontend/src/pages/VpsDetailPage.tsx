import React, { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import Typography from '@mui/material/Typography';
import Paper from '@mui/material/Paper';
import Grid from '@mui/material/Grid'; // Explicit import
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ButtonGroup from '@mui/material/ButtonGroup';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { getVpsMetricsTimeseries } from '../services/metricsService';
import { getVpsDetail } from '../services/vpsService'; // Added
import type { PerformanceMetricPoint, VpsListItemResponse } from '../types'; // Added VpsListItemResponse
import type { NameType, ValueType } from 'recharts/types/component/DefaultTooltipContent';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemText from '@mui/material/ListItemText';
import Divider from '@mui/material/Divider';
import Chip from '@mui/material/Chip';

// Helper to format date for XAxis
const formatDateTick = (tickItem: string) => {
  const date = new Date(tickItem);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
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


type TimeRangeOption = '1h' | '6h' | '24h' | '7d';

const VpsDetailPage: React.FC = () => {
const { vpsId } = useParams<{ vpsId: string }>();
const [vpsDetail, setVpsDetail] = useState<VpsListItemResponse | null>(null); // Added state for VPS details
const [cpuData, setCpuData] = useState<PerformanceMetricPoint[]>([]);
const [memoryData, setMemoryData] = useState<PerformanceMetricPoint[]>([]);
const [networkData, setNetworkData] = useState<PerformanceMetricPoint[]>([]);
const [loading, setLoading] = useState(true);
const [loadingMetrics, setLoadingMetrics] = useState(true); // Separate loading for metrics
const [error, setError] = useState<string | null>(null);
const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('1h');

  const timeRangeToMillis: Record<TimeRangeOption, number> = {
    '1h': 60 * 60 * 1000,
    '6h': 6 * 60 * 60 * 1000,
    '24h': 24 * 60 * 60 * 1000,
    '7d': 7 * 24 * 60 * 60 * 1000,
  };

  const intervalMap: Record<TimeRangeOption, string> = {
    '1h': '1m', // 1 minute interval for 1 hour
    '6h': '5m', // 5 minutes interval for 6 hours
    '24h': '15m', // 15 minutes interval for 24 hours
    '7d': '1h', // 1 hour interval for 7 days
  };

  useEffect(() => {
    if (!vpsId) {
      setError('VPS ID not found.');
      setLoading(false);
      setLoadingMetrics(false);
      return;
    }

    const fetchVpsDetails = async () => {
      setLoading(true);
      setError(null);
      try {
        const details = await getVpsDetail(vpsId);
        setVpsDetail(details);
      } catch (err) {
        console.error('Failed to fetch VPS details:', err);
        setError('无法加载VPS详细信息。请稍后再试。');
      } finally {
        setLoading(false);
      }
    };

    fetchVpsDetails();
  }, [vpsId]);

  useEffect(() => {
    if (!vpsId) return;

    const fetchMetricsData = async () => {
      setLoadingMetrics(true);
      // setError(null); // Keep previous errors unless specifically for metrics
      try {
        const endTime = new Date();
        const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
        const interval = intervalMap[selectedTimeRange];

        const metrics = await getVpsMetricsTimeseries(
          vpsId,
          startTime.toISOString(),
          endTime.toISOString(),
          interval
        );

        const cpuPoints: PerformanceMetricPoint[] = [];
        const memoryPoints: PerformanceMetricPoint[] = [];
        const networkPoints: PerformanceMetricPoint[] = [];

        metrics.forEach(point => {
          const cpuValue = point.avg_cpu_usage_percent ?? point.cpu_usage_percent;
          if (cpuValue != null) {
            cpuPoints.push({ time: point.time, vps_id: point.vps_id, cpu_usage_percent: cpuValue });
          }
          const memoryUsagePercent = calculateMemoryUsagePercent(point);
          if (memoryUsagePercent != null) {
            memoryPoints.push({ time: point.time, vps_id: point.vps_id, memory_usage_percent: memoryUsagePercent });
          }
          if (point.avg_network_rx_instant_bps != null || point.avg_network_tx_instant_bps != null) {
            networkPoints.push({
              time: point.time,
              vps_id: point.vps_id,
              avg_network_rx_instant_bps: point.avg_network_rx_instant_bps,
              avg_network_tx_instant_bps: point.avg_network_tx_instant_bps,
            });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
        setNetworkData(networkPoints);
      } catch (err) {
        console.error('Failed to fetch VPS metrics timeseries:', err);
        // setError('无法加载VPS指标数据。请稍后再试。'); // Avoid overwriting detail fetch error
        if (!error) setError('无法加载VPS指标数据。请稍后再试。');
      } finally {
        setLoadingMetrics(false);
      }
    };

    fetchMetricsData();
  }, [vpsId, selectedTimeRange, error]); // Added error to dependency to avoid loop if detail fails

  const handleRefresh = () => {
    if (!vpsId) return;
    // Refetch both details and metrics
    const fetchDetailsForRefresh = async () => {
      setLoading(true);
      setError(null); // Clear previous errors
      try {
        const details = await getVpsDetail(vpsId);
        setVpsDetail(details);
      } catch (err) {
        console.error('Failed to refresh VPS details:', err);
        setError('刷新VPS详细信息失败。');
      } finally {
        setLoading(false);
      }
    };

    const fetchMetricsForRefresh = async () => {
      setLoadingMetrics(true);
      try {
        const endTime = new Date();
        const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
        const interval = intervalMap[selectedTimeRange];
        const metrics = await getVpsMetricsTimeseries(vpsId, startTime.toISOString(), endTime.toISOString(), interval);
        const cpuPoints: PerformanceMetricPoint[] = [];
        const memoryPoints: PerformanceMetricPoint[] = [];
        const networkPoints: PerformanceMetricPoint[] = [];
        metrics.forEach(point => {
          const cpuValue = point.avg_cpu_usage_percent ?? point.cpu_usage_percent;
          if (cpuValue != null) cpuPoints.push({ time: point.time, vps_id: point.vps_id, cpu_usage_percent: cpuValue });
          const memUsagePct = calculateMemoryUsagePercent(point);
          if (memUsagePct != null) memoryPoints.push({ time: point.time, vps_id: point.vps_id, memory_usage_percent: memUsagePct });
          if (point.avg_network_rx_instant_bps != null || point.avg_network_tx_instant_bps != null) {
            networkPoints.push({
              time: point.time, vps_id: point.vps_id,
              avg_network_rx_instant_bps: point.avg_network_rx_instant_bps,
              avg_network_tx_instant_bps: point.avg_network_tx_instant_bps,
            });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
        setNetworkData(networkPoints);
      } catch (err) {
        console.error('Failed to refresh VPS metrics timeseries:', err);
        if (!error) setError('刷新指标数据失败。');
      } finally {
        setLoadingMetrics(false);
      }
    };
    fetchDetailsForRefresh(); // Fetch details first
    fetchMetricsForRefresh(); // Then fetch metrics
  };


  if (loading) { // Initial loading for VPS details
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: 'calc(100vh - 200px)' }}>
        <CircularProgress />
      </Box>
    );
  }

  if (error && !vpsDetail) { // If error and no vpsDetail, show full page error
    return <Alert severity="error" sx={{ m: 2 }}>{error}</Alert>;
  }
  
  if (!vpsDetail) { // Should not happen if not loading and no error, but as a fallback
    return <Alert severity="info" sx={{ m: 2 }}>VPS详情未找到。</Alert>;
  }

  // Custom Tooltip Formatter for Network Speed
  const networkTooltipFormatter = (value: ValueType, name: NameType): [string, NameType] => {
    const formattedValue = formatNetworkSpeed(value as number);
    return [formattedValue, name];
  };

  return (
    <Box sx={{ width: '100%', maxWidth: 1200, margin: 'auto', p: 2 }}>
      <Typography variant="h4" gutterBottom>
        {vpsDetail.name} (ID: {vpsDetail.id})
      </Typography>

      {/* VPS Info Section */}
      <Paper sx={{ p: 2, mb: 3 }}>
        <Typography variant="h6" gutterBottom>服务器信息</Typography>
        <List dense>
          <ListItem>
            <ListItemText primary="状态" secondary={
              <Chip
                label={vpsDetail.status?.toUpperCase()}
                color={vpsDetail.status === 'online' ? 'success' : vpsDetail.status === 'offline' ? 'error' : 'default'}
                size="small"
              />
            } />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText primary="运行时间" secondary={formatUptime(vpsDetail.latestMetrics?.uptimeSeconds)} />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText
              primary="操作系统"
              secondary={`${vpsDetail.metadata?.os_family || vpsDetail.osType || (vpsDetail.metadata?.os_name as string) || 'N/A'} (${(vpsDetail.metadata?.os_version as string) || 'N/A'})`}
            />
          </ListItem>
          <Divider component="li" />
           <ListItem>
            <ListItemText primary="内核版本" secondary={`${(vpsDetail.metadata?.kernel_version as string) || 'N/A'}`} />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText primary="架构" secondary={`${(vpsDetail.metadata?.architecture as string) || (vpsDetail.metadata?.arch as string) || 'N/A'}`} />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText primary="CPU型号" secondary={`${(vpsDetail.metadata?.cpu_model as string) || 'N/A'}`} />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText primary="内存大小" secondary={formatBytes(vpsDetail.latestMetrics?.memoryTotalBytes)} />
          </ListItem>
          <Divider component="li" />
          <ListItem>
            <ListItemText
              primary="磁盘大小"
              secondary={`${formatBytes(vpsDetail.latestMetrics?.diskUsedBytes)} / ${formatBytes(vpsDetail.latestMetrics?.diskTotalBytes)}`}
            />
          </ListItem>
        </List>
      </Paper>

      {error && <Alert severity="warning" sx={{ mb: 2 }}>{error}</Alert>}


      <Box sx={{ mb: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <ButtonGroup variant="outlined" aria-label="Time range selection">
          {(Object.keys(timeRangeToMillis) as TimeRangeOption[]).map((range) => (
            <Button
              key={range}
              onClick={() => setSelectedTimeRange(range)}
              variant={selectedTimeRange === range ? "contained" : "outlined"}
            >
              {range.toUpperCase()}
            </Button>
          ))}
        </ButtonGroup>
        <Button variant="outlined" onClick={handleRefresh} disabled={loading || loadingMetrics}>
          刷新
        </Button>
      </Box>

      {loadingMetrics && (
         <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '200px' }}>
           <CircularProgress />
         </Box>
      )}
      {!loadingMetrics && (
        <Grid container spacing={3}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Paper sx={{ p: 2, height: 350, display: 'flex', flexDirection: 'column' }}>
              <Typography variant="h6" gutterBottom align="center">CPU 使用率 (%)</Typography>
              {cpuData.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={cpuData} margin={{ top: 5, right: 20, left: -20, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" tickFormatter={formatDateTick} />
                    <YAxis domain={[0, 100]} />
                    <Tooltip />
                    <Legend />
                    <Line type="monotone" dataKey="cpu_usage_percent" stroke="#8884d8" dot={false} name="CPU %" />
                  </LineChart>
                </ResponsiveContainer>
              ) : (
                <Box sx={{ flexGrow: 1, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
                  <Typography>暂无CPU数据。</Typography>
                </Box>
              )}
            </Paper>
          </Grid>
          <Grid size={{ xs: 12, md: 6 }}>
            <Paper sx={{ p: 2, height: 350, display: 'flex', flexDirection: 'column' }}>
              <Typography variant="h6" gutterBottom align="center">内存使用率 (%)</Typography>
              {memoryData.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={memoryData} margin={{ top: 5, right: 20, left: -20, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" tickFormatter={formatDateTick} />
                    <YAxis domain={[0, 100]} />
                    <Tooltip />
                    <Legend />
                    <Line type="monotone" dataKey="memory_usage_percent" stroke="#82ca9d" dot={false} name="内存 %" />
                  </LineChart>
                </ResponsiveContainer>
              ) : (
                <Box sx={{ flexGrow: 1, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
                  <Typography>暂无内存数据。</Typography>
                </Box>
              )}
            </Paper>
          </Grid>
          {/* Network Chart */}
          <Grid size={{ xs: 12 }}> {/* Full width for network chart */}
            <Paper sx={{ p: 2, height: 350, display: 'flex', flexDirection: 'column' }}>
              <Typography variant="h6" gutterBottom align="center">网络速率</Typography>
              {networkData.length > 0 ? (
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={networkData} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" tickFormatter={formatDateTick} />
                    {/* Adjust YAxis tick formatter for network speed */}
                    <YAxis tickFormatter={formatNetworkSpeed} width={80} />
                    {/* Adjust Tooltip formatter */}
                    <Tooltip formatter={networkTooltipFormatter} />
                    <Legend />
                    <Line type="monotone" dataKey="avg_network_rx_instant_bps" stroke="#ff7300" dot={false} name="下载速率" />
                    <Line type="monotone" dataKey="avg_network_tx_instant_bps" stroke="#387908" dot={false} name="上传速率" />
                  </LineChart>
                </ResponsiveContainer>
              ) : (
                <Box sx={{ flexGrow: 1, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
                  <Typography>暂无网络数据。</Typography>
                </Box>
              )}
            </Paper>
          </Grid>
        </Grid>
      )}
    </Box>
  );
};

export default VpsDetailPage;