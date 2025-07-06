import React, { useMemo, useEffect, useState, useRef } from 'react';
import { AreaChart, Area, LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend as RechartsLegend } from 'recharts';
import { useTranslation } from 'react-i18next';
import { getLatestNMetrics } from '../services/metricsService';
import { getMonitorResultsByVpsId } from '../services/serviceMonitorService';
import type { PerformanceMetricPoint, ServiceMonitorResult } from '../types';
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import websocketService from '../services/websocketService';
import { type ChartConfig } from "@/components/ui/chart";

interface VpsMetricsChartProps {
  vpsId: number;
  initialMetrics?: PerformanceMetricPoint | null;
}

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];

// --- Service Monitor Chart Component ---
const ServiceMonitorChart: React.FC<{ vpsId: number }> = ({ vpsId }) => {
  const { t } = useTranslation();
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [monitorIds, setMonitorIds] = useState<Set<number>>(new Set());
  const newResultsBuffer = useRef<ServiceMonitorResult[]>([]);

  useEffect(() => {
    const fetchInitialData = async () => {
      try {
        setLoading(true);
        const data = await getMonitorResultsByVpsId(vpsId, undefined, undefined, 100); // Fetch last 100 points per monitor
        const sortedData = data.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
        setResults(sortedData);
        const ids = new Set(data.map(r => r.monitorId));
        setMonitorIds(ids);
      } catch (error) {
        console.error(`Failed to fetch service monitor results:`, error);
      } finally {
        setLoading(false);
      }
    };
    fetchInitialData();
  }, [vpsId]);

  useEffect(() => {
    const handleNewResult = (result: ServiceMonitorResult) => {
      if (monitorIds.has(result.monitorId)) {
        newResultsBuffer.current.push(result);
      }
    };

    websocketService.on('service_monitor_result', handleNewResult);
    return () => {
      websocketService.off('service_monitor_result', handleNewResult);
    };
  }, [monitorIds]);

  useEffect(() => {
    const intervalId = setInterval(() => {
      if (newResultsBuffer.current.length > 0) {
        setResults(prevResults => {
          const updatedResults = [...prevResults, ...newResultsBuffer.current]
            .sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
          newResultsBuffer.current = [];
          return updatedResults.length > 500 ? updatedResults.slice(updatedResults.length - 500) : updatedResults;
        });
      }
    }, 2000); // Update every 2 seconds

    return () => clearInterval(intervalId);
  }, []);

  const { chartData, monitorLines } = useMemo(() => {
    if (results.length === 0) {
      return { chartData: [], monitorLines: [] };
    }

    const groupedByMonitor = results.reduce((acc, result) => {
      const key = result.monitorName;
      if (!acc[key]) {
        acc[key] = [];
      }
      acc[key].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    const dataKeys = Object.keys(groupedByMonitor);
    const config: ChartConfig = {};
    const lines = dataKeys.map((monitorName, index) => {
      config[monitorName] = {
        label: monitorName,
        color: AGENT_COLORS[index % AGENT_COLORS.length],
      };
      return {
        dataKey: monitorName,
        name: monitorName,
        stroke: AGENT_COLORS[index % AGENT_COLORS.length],
      };
    });

    const timePoints = [...new Set(results.map(r => new Date(r.time).getTime()))].sort((a, b) => a - b);
    
    const data = timePoints.map(timeNum => {
        const point: { time: string; [key: string]: number | string | null } = {
            time: new Date(timeNum).toISOString(),
        };
        for (const monitorName of dataKeys) {
            const latestResultForTime = groupedByMonitor[monitorName]
                .filter(r => new Date(r.time).getTime() <= timeNum)
                .pop();
            
            if (latestResultForTime && latestResultForTime.isUp) {
                point[monitorName] = latestResultForTime.latencyMs;
            } else {
                point[monitorName] = null;
            }
        }
        return point;
    });

    return { chartData: data, monitorLines: lines };
  }, [results]);

  if (loading && chartData.length === 0) {
    return <div className="h-24 w-full flex items-center justify-center text-muted-foreground text-sm">{t('common.status.loading')}...</div>;
  }

  if (chartData.length === 0) {
    return <div className="h-24 w-full flex items-center justify-center text-muted-foreground text-sm">{t('vps.noServiceMonitors')}</div>;
  }

  return (
    <div className="h-24 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData} margin={{ top: 5, right: 10, left: -20, bottom: 20 }}>
          <CartesianGrid strokeDasharray="3 3" vertical={false} />
          <XAxis dataKey="time" hide />
          <YAxis tickFormatter={(tick) => `${tick}ms`} domain={[0, 'dataMax + 50']} tick={{ fontSize: 10 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--background) / 0.8)',
              backdropFilter: 'blur(2px)',
              borderRadius: 'var(--radius)',
              fontSize: '0.75rem',
              padding: '4px 8px',
            }}
            labelFormatter={(label) => new Date(label).toLocaleTimeString()}
            formatter={(value: number, name: string) => [`${value.toFixed(0)}ms`, name]}
          />
          <RechartsLegend content={<ChartLegendContent />} />
          {monitorLines.map(line => (
            <Line
              key={line.dataKey}
              type="monotone"
              dataKey={line.dataKey}
              stroke={line.stroke}
              strokeWidth={2}
              dot={false}
              connectNulls
              isAnimationActive={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
};

// --- CPU/RAM Chart Component ---
const PerformanceChart: React.FC<{ vpsId: number; metricType: 'cpu' | 'ram'; initialMetrics: PerformanceMetricPoint | null }> = ({ vpsId, metricType, initialMetrics }) => {
  const { t } = useTranslation();
  const [metrics, setMetrics] = useState<PerformanceMetricPoint[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        setLoading(true);
        const data = await getLatestNMetrics(vpsId.toString(), 60);
        setMetrics(data);
      } catch (error) {
        console.error(`Failed to fetch ${metricType} metrics:`, error);
      } finally {
        setLoading(false);
      }
    };
    fetchMetrics();
  }, [vpsId, metricType]);

  useEffect(() => {
    if (!initialMetrics) return;
    setMetrics(prevMetrics => {
      const exists = prevMetrics.some(m => m.time === initialMetrics.time);
      if (exists) return prevMetrics;
      const updatedMetrics = [...prevMetrics, initialMetrics].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
      return updatedMetrics.length > 60 ? updatedMetrics.slice(updatedMetrics.length - 60) : updatedMetrics;
    });
  }, [initialMetrics]);

  const chartData = useMemo(() => {
    const calculateMemoryUsagePercent = (dataPoint: Partial<PerformanceMetricPoint>): number | null => {
      if (dataPoint.memoryUsageBytes != null && dataPoint.memoryTotalBytes != null && dataPoint.memoryTotalBytes > 0) {
        return (dataPoint.memoryUsageBytes / dataPoint.memoryTotalBytes) * 100;
      }
      return null;
    };
    return metrics.map(point => ({
      time: point.time,
      usage: metricType === 'cpu' ? point.cpuUsagePercent : calculateMemoryUsagePercent(point),
    })).filter(p => p.usage !== null);
  }, [metrics, metricType]);

  if (loading && chartData.length === 0) {
    return <div className="h-24 w-full flex items-center justify-center text-muted-foreground text-sm">{t('loading')}...</div>;
  }

  if (chartData.length === 0) {
    return <div className="h-24 w-full flex items-center justify-center text-muted-foreground text-sm">{t('vps.noData')}</div>;
  }

  const gradientId = `color${metricType}`;
  const strokeColor = metricType === 'cpu' ? 'hsl(var(--chart-1))' : 'hsl(var(--chart-2))';

  return (
    <div className="h-24 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 5, right: 10, left: -20, bottom: 0 }}>
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={strokeColor} stopOpacity={0.4} />
              <stop offset="95%" stopColor={strokeColor} stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis dataKey="time" hide />
          <YAxis tickFormatter={(tick) => `${tick.toFixed(0)}%`} domain={[0, 100]} tick={{ fontSize: 10 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--background) / 0.8)',
              backdropFilter: 'blur(2px)',
              borderRadius: 'var(--radius)',
              fontSize: '0.75rem',
              padding: '4px 8px',
            }}
            labelFormatter={(label) => new Date(label).toLocaleTimeString()}
            formatter={(value: number) => [`${value.toFixed(1)}%`, t(`vps.${metricType}`)]}
          />
          <Area type="monotone" dataKey="usage" stroke={strokeColor} fillOpacity={1} fill={`url(#${gradientId})`} strokeWidth={2} dot={false} isAnimationActive={false} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
};

// --- Main Component with Tabs ---
export const VpsMetricsChart: React.FC<VpsMetricsChartProps> = ({ vpsId, initialMetrics }) => {
  const { t } = useTranslation();

  return (
    <Tabs defaultValue="service" className="w-full">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="service">{t('vps.serviceMonitor')}</TabsTrigger>
        <TabsTrigger value="cpu">{t('vps.cpu')}</TabsTrigger>
        <TabsTrigger value="ram">{t('vps.ram')}</TabsTrigger>
      </TabsList>
      <TabsContent value="service">
        <ServiceMonitorChart vpsId={vpsId} />
      </TabsContent>
      <TabsContent value="cpu">
        <PerformanceChart vpsId={vpsId} metricType="cpu" initialMetrics={initialMetrics ?? null} />
      </TabsContent>
      <TabsContent value="ram">
        <PerformanceChart vpsId={vpsId} metricType="ram" initialMetrics={initialMetrics ?? null} />
      </TabsContent>
    </Tabs>
  );
};

// Custom Legend for Service Monitor Chart
interface LegendPayload {
  value: string;
  color: string;
}

interface ChartLegendContentProps {
    payload?: LegendPayload[];
}

const ChartLegendContent: React.FC<ChartLegendContentProps> = ({ payload }) => {
  return (
    <div className="flex items-center justify-center gap-x-4 gap-y-2 flex-wrap -mt-4">
      {payload?.map((entry, index) => (
        <div key={`item-${index}`} className="flex items-center gap-1.5 cursor-pointer">
          <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: entry.color }} />
          <span className="text-xs text-muted-foreground">{entry.value}</span>
        </div>
      ))}
    </div>
  );
};