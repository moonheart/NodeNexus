import { useState, useEffect, useRef } from 'react';
import { useServerListStore } from '@/store/serverListStore';
import { getVpsMetrics } from '@/services/metricsService';
import { getMonitorResults } from '@/services/serviceMonitorService';
import { getTimeRangeDetails, type TimeRangeValue } from '@/components/TimeRangeSelector';
import type { PerformanceMetricPoint, ServiceMonitorResult } from '@/types';

// --- Types ---

export type ChartSourceType = 'vps' | 'monitor';
export type ChartViewMode = 'realtime' | 'historical';

// A generic data point structure that charts can use
export type ChartDataPoint = {
  time: number;
  [key: string]: number | null | undefined;
};

import type { MetricType } from '@/utils/chartConfigFactory';

interface UseMetricsProps {
  sourceType: ChartSourceType;
  sourceId: number;
  metricType: MetricType;
  viewMode: ChartViewMode;
  timeRange?: TimeRangeValue;
  // Keep previous data visible while loading new data
  preserveDataOnFetch?: boolean;
  // Conditionally enable/disable the hook
  enabled?: boolean;
}

// --- Helper Functions ---

const calculateMemoryUsagePercent = (p: PerformanceMetricPoint): number | null => {
  if (p.memoryUsageBytes != null && p.memoryTotalBytes != null && p.memoryTotalBytes > 0) {
    return (p.memoryUsageBytes / p.memoryTotalBytes) * 100;
  }
  return p.memoryUsagePercent ?? null; // Fallback to provided percent if available
};

const transformVpsData = (points: PerformanceMetricPoint[]): ChartDataPoint[] => {
  return points.map(p => ({
    time: new Date(p.time).getTime(),
    // Selectively pick the properties needed for charts to avoid polluting the data object
    cpuUsagePercent: p.cpuUsagePercent,
    memoryUsagePercent: calculateMemoryUsagePercent(p),
    networkRxInstantBps: p.networkRxInstantBps,
    networkTxInstantBps: p.networkTxInstantBps,
    diskIoReadBps: p.diskIoReadBps,
    diskIoWriteBps: p.diskIoWriteBps,
  }));
};

const transformMonitorData = (results: ServiceMonitorResult[], groupBy: 'monitorName' | 'agentName' = 'monitorName'): ChartDataPoint[] => {
    const groupedByTime = results.reduce((acc, r) => {
        const time = new Date(r.time).getTime();
        if (!acc[time]) {
            acc[time] = { time };
        }
        const key = groupBy === 'agentName' ? r.agentName : r.monitorName;
        acc[time][key] = r.isUp ? r.latencyMs : null;
        return acc;
    }, {} as Record<number, ChartDataPoint>);

    return Object.values(groupedByTime).sort((a, b) => a.time - b.time);
};


// --- The Hook ---

export const useMetrics = ({
  sourceType,
  sourceId,
  metricType,
  viewMode,
  timeRange = '1h',
  preserveDataOnFetch = true,
  enabled = true,
}: UseMetricsProps) => {
  const [data, setData] = useState<ChartDataPoint[]>([]);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const previousData = useRef<ChartDataPoint[]>([]);
  const viewModeRef = useRef(viewMode);
  useEffect(() => {
    viewModeRef.current = viewMode;
  }, [viewMode]);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      setData([]);
      return;
    }

    let isMounted = true;
    let unsubscribe: (() => void) | null = null;

    const fetchData = async () => {
      if (preserveDataOnFetch) {
        previousData.current = data;
      }
      setLoading(true);
      setError(null);

      try {
        // --- VPS Data Source ---
        if (sourceType === 'vps') {
          // --- Service Latency for a VPS ---
          if (metricType === 'service-latency') {
            if (viewMode === 'realtime') {
              const { getInitialVpsMonitorResults, subscribeToVpsMonitorResults } = useServerListStore.getState();
              const initialData = await getInitialVpsMonitorResults(sourceId);
              if (isMounted) setData(transformMonitorData(initialData));

              unsubscribe = subscribeToVpsMonitorResults(sourceId, (newResults) => {
                if (viewModeRef.current === 'realtime') {
                  // The store now pushes the full, filtered array, so we can just replace the data.
                  setData(transformMonitorData(newResults));
                }
              });
            } else { // Historical
              const { getMonitorResultsByVpsId } = await import('@/services/serviceMonitorService');
              const timeDetails = getTimeRangeDetails(timeRange);
              const historicalData = await getMonitorResultsByVpsId(sourceId, timeDetails.startTime, timeDetails.endTime, timeDetails.interval);
              if (isMounted) setData(transformMonitorData(historicalData));
            }
          }
          // --- Performance Metrics for a VPS (cpu, ram, etc.) ---
          else {
            if (viewMode === 'realtime') {
              const store = useServerListStore.getState();
              const existingMetrics = store.initialVpsMetrics[sourceId];
              
              // If we already have data, use it immediately. Otherwise, fetch it.
              if (existingMetrics && existingMetrics.status === 'success') {
                setData(transformVpsData(existingMetrics.data));
              } else {
                await store.ensureInitialVpsPerformanceMetrics(sourceId);
                if (!isMounted) return;
                const initialData = store.initialVpsMetrics[sourceId]?.data || [];
                setData(transformVpsData(initialData));
              }
              
              unsubscribe = useServerListStore.subscribe((state) => {
                if (viewModeRef.current === 'realtime') {
                  const newData = state.initialVpsMetrics[sourceId]?.data;
                  if (newData) {
                    // The store now provides pre-filtered data.
                    setData(transformVpsData(newData));
                  }
                }
              });
            } else { // Historical
              const timeDetails = getTimeRangeDetails(timeRange);
              const historicalData = await getVpsMetrics(sourceId, timeDetails.startTime, timeDetails.endTime, timeDetails.interval);
              if (isMounted) setData(transformVpsData(historicalData));
            }
          }
        }
        // --- Single Monitor Data Source ---
        else { // sourceType === 'monitor'
          if (viewMode === 'realtime') {
            const { getInitialMonitorResults, subscribeToMonitorResults } = useServerListStore.getState();
            const endTime = new Date();
            const startTime = new Date(endTime.getTime() - 10 * 60 * 1000); // Last 10 mins for initial load
            const initialData = await getInitialMonitorResults(sourceId, startTime.toISOString(), endTime.toISOString(), null);
            if (isMounted) setData(transformMonitorData(initialData, 'agentName'));
            unsubscribe = subscribeToMonitorResults(sourceId, (newResults) => {
              if (viewModeRef.current === 'realtime') {
                setData(prevData => [...prevData, ...transformMonitorData(newResults, 'agentName')].slice(-1000));
              }
            });
          } else { // Historical
            const timeDetails = getTimeRangeDetails(timeRange);
            const historicalData = await getMonitorResults(sourceId, timeDetails.startTime, timeDetails.endTime, timeDetails.interval);
            if (isMounted) setData(transformMonitorData(historicalData, 'agentName'));
          }
        }
      } catch (err) {
        console.error(`Failed to fetch metrics for ${sourceType} ${sourceId}:`, err);
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
  }, [sourceType, sourceId, metricType, viewMode, timeRange, preserveDataOnFetch, enabled]);

  const displayData = loading && preserveDataOnFetch ? previousData.current : data;

  return { data: displayData, loading, error };
};