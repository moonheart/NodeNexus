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
import type { PerformanceMetricPoint } from '../types';

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

type TimeRangeOption = '1h' | '6h' | '24h' | '7d';

const VpsDetailPage: React.FC = () => {
  const { vpsId } = useParams<{ vpsId: string }>();
  const [cpuData, setCpuData] = useState<PerformanceMetricPoint[]>([]);
  const [memoryData, setMemoryData] = useState<PerformanceMetricPoint[]>([]);
  const [loading, setLoading] = useState(true);
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
      return;
    }

    const fetchData = async () => {
      setLoading(true);
      setError(null);
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

        metrics.forEach(point => {
          // For CPU, prefer aggregated if available, else raw
          const cpuValue = point.avg_cpu_usage_percent ?? point.cpu_usage_percent;
          if (cpuValue != null) {
            cpuPoints.push({
              time: point.time,
              vps_id: point.vps_id,
              cpu_usage_percent: cpuValue,
            });
          }

          // For Memory, calculate percentage and store
          const memoryUsagePercent = calculateMemoryUsagePercent(point);
          if (memoryUsagePercent != null) {
            memoryPoints.push({
              time: point.time,
              vps_id: point.vps_id,
              memory_usage_percent: memoryUsagePercent,
            });
          }
        });

        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);

      } catch (err) {
        console.error('Failed to fetch VPS metrics:', err);
        setError('无法加载VPS指标数据。请稍后再试。');
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [vpsId, selectedTimeRange]); // fetchData will be memoized by useEffect if its dependencies don't change

  const handleRefresh = () => {
    if (!vpsId) return;
    // Directly call fetchData, it will use the current selectedTimeRange
    const fetchDataForRefresh = async () => {
      setLoading(true);
      setError(null);
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

        metrics.forEach(point => {
          const cpuValue = point.avg_cpu_usage_percent ?? point.cpu_usage_percent;
          if (cpuValue != null) {
            cpuPoints.push({
              time: point.time,
              vps_id: point.vps_id,
              cpu_usage_percent: cpuValue,
            });
          }
          const memoryUsagePercent = calculateMemoryUsagePercent(point);
          if (memoryUsagePercent != null) {
            memoryPoints.push({
              time: point.time,
              vps_id: point.vps_id,
              memory_usage_percent: memoryUsagePercent,
            });
          }
        });
        setCpuData(cpuPoints);
        setMemoryData(memoryPoints);
      } catch (err) {
        console.error('Failed to refresh VPS metrics:', err);
        setError('刷新指标数据失败。');
      } finally {
        setLoading(false);
      }
    };
    fetchDataForRefresh();
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return <Alert severity="error">{error}</Alert>;
  }

  return (
    <Box sx={{ width: '100%', maxWidth: 1200, margin: 'auto', p: 2 }}>
      <Typography variant="h4" gutterBottom>
        VPS 详情 (ID: {vpsId})
      </Typography>

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
        <Button variant="outlined" onClick={handleRefresh} disabled={loading}>
          刷新
        </Button>
      </Box>

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
      </Grid>
    </Box>
  );
};

export default VpsDetailPage;