import { useState, useEffect } from 'react';
import { useServerListStore } from '@/store/serverListStore';
import { getVpsMetrics } from '@/services/metricsService';
import { getTimeRangeDetails, type TimeRangeValue } from '@/components/TimeRangeSelector';
import type { PerformanceMetricPoint } from '@/types';
import type { ChartDataPoint, ChartViewMode } from './useMetrics';

// This helper function is duplicated from useMetrics.ts.
// Consider moving it to a shared utility file in the future.
const calculateMemoryUsagePercent = (p: PerformanceMetricPoint): number | null => {
  if (p.memoryUsageBytes != null && p.memoryTotalBytes != null && p.memoryTotalBytes > 0) {
    return (p.memoryUsageBytes / p.memoryTotalBytes) * 100;
  }
  return p.memoryUsagePercent ?? null;
};

const transformVpsData = (points: PerformanceMetricPoint[]): ChartDataPoint[] => {
  return points.map(p => ({
    time: new Date(p.time).getTime(),
    cpuUsagePercent: p.cpuUsagePercent,
    memoryUsagePercent: calculateMemoryUsagePercent(p),
    networkRxInstantBps: p.networkRxInstantBps,
    networkTxInstantBps: p.networkTxInstantBps,
    diskIoReadBps: p.diskIoReadBps,
    diskIoWriteBps: p.diskIoWriteBps,
  }));
};

interface UseVpsPerformanceMetricsProps {
  vpsId: number;
  viewMode: ChartViewMode;
  timeRange?: TimeRangeValue;
  enabled?: boolean;
}

/**
 * A hook dedicated to fetching all performance metrics for a single VPS.
 * This avoids multiple API calls when displaying several charts for the same VPS and time range.
 */
export const useVpsPerformanceMetrics = ({
  vpsId,
  viewMode,
  timeRange = '1h',
  enabled = true,
}: UseVpsPerformanceMetricsProps) => {
  const [data, setData] = useState<ChartDataPoint[]>([]);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      setData([]);
      return;
    }
    let isMounted = true;
    let unsubscribe: (() => void) | null = null;

    const fetchData = async () => {
      setLoading(true);
      setError(null);

      try {
        if (viewMode === 'realtime') {
          await useServerListStore.getState().ensureInitialVpsPerformanceMetrics(vpsId);
          if (!isMounted) return;

          const initialData = useServerListStore.getState().initialVpsMetrics[vpsId]?.data || [];
          setData(transformVpsData(initialData));

          unsubscribe = useServerListStore.subscribe(
            (state) => {
              const newData = state.initialVpsMetrics[vpsId]?.data;
              if (newData) {
                setData(transformVpsData(newData));
              }
            }
          );
        } else { // Historical
          const timeDetails = getTimeRangeDetails(timeRange);
          const historicalData = await getVpsMetrics(vpsId, timeDetails.startTime, timeDetails.endTime, timeDetails.interval);
          if (isMounted) {
            setData(transformVpsData(historicalData));
          }
        }
      } catch (err) {
        console.error(`Failed to fetch performance metrics for VPS ${vpsId}:`, err);
        if (isMounted) setError('Failed to fetch data.');
      } finally {
        if (isMounted) setLoading(false);
      }
    };

    fetchData();

    return () => {
      isMounted = false;
      if (unsubscribe) {
        unsubscribe();
      }
    };
  }, [vpsId, viewMode, timeRange, enabled]);

  return { data, loading, error };
};