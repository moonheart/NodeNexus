import React, { useMemo, useEffect, useState } from 'react';
import { LineChart, Line, XAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend as RechartsLegend } from 'recharts';
import { useTranslation } from 'react-i18next';
import type { ServiceMonitorResult } from '../types';
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useServerListStore, type UnsubscribeFunction } from '../store/serverListStore';
import { type ChartConfig } from "@/components/ui/chart";
import RealtimeMetricChart from './RealtimeMetricChart';

interface VpsMetricsChartProps {
  vpsId: number;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];

// --- Service Monitor Chart Component (Existing, no changes needed for this task) ---
const ServiceMonitorChart: React.FC<{ vpsId: number }> = React.memo(({ vpsId }) => {
  const { t } = useTranslation();
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [loading, setLoading] = useState(true);
  const { getInitialVpsMonitorResults, subscribeToVpsMonitorResults } = useServerListStore();

  useEffect(() => {
    let isMounted = true;
    let unsubscribe: UnsubscribeFunction | null = null;

    const setup = async () => {
      try {
        setLoading(true);
        const initialData = await getInitialVpsMonitorResults(vpsId);
        if (!isMounted) return;

        setResults(initialData);

        unsubscribe = subscribeToVpsMonitorResults(vpsId, (newResults) => {
          if (!isMounted) return;
          setResults(prevResults => {
            const updated = [...prevResults, ...newResults];
            const byMonitor = updated.reduce((acc, r) => {
              if (!acc[r.monitorId]) acc[r.monitorId] = [];
              acc[r.monitorId].push(r);
              return acc;
            }, {} as Record<number, ServiceMonitorResult[]>);

            const final = Object.values(byMonitor).flatMap(monitorResults =>
              monitorResults
                .sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime())
                .slice(-100)
            );
            
            return final.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
          });
        });

      } catch (error) {
        if (isMounted) console.error(`Failed to fetch or subscribe to service monitor results for VPS ${vpsId}:`, error);
      } finally {
        if (isMounted) setLoading(false);
      }
    };

    setup();

    return () => {
      isMounted = false;
      if (unsubscribe) {
        unsubscribe();
      }
    };
  }, [vpsId, getInitialVpsMonitorResults, subscribeToVpsMonitorResults]);

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
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--background) / 0.8)',
              backdropFilter: 'blur(8px)',
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
});
ServiceMonitorChart.displayName = 'ServiceMonitorChart';

// --- PerformanceChart component removed ---

// --- Main Component with Tabs ---
export const VpsMetricsChart: React.FC<VpsMetricsChartProps> = ({ vpsId, activeTab, onTabChange }) => {
  const { t } = useTranslation();

  return (
    <Tabs value={activeTab} onValueChange={onTabChange} className="w-full">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="service">{t('vps.serviceMonitor')}</TabsTrigger>
        <TabsTrigger value="cpu">{t('vps.cpu')}</TabsTrigger>
        <TabsTrigger value="ram">{t('vps.ram')}</TabsTrigger>
      </TabsList>
      <TabsContent value="service">
        <ServiceMonitorChart vpsId={vpsId} />
      </TabsContent>
      <TabsContent value="cpu">
        <RealtimeMetricChart vpsId={vpsId} metricType="cpu" showTitle={false} showYAxis={false} showXAxis={false} className="h-24 w-full" />
      </TabsContent>
      <TabsContent value="ram">
        <RealtimeMetricChart vpsId={vpsId} metricType="ram" showTitle={false} showYAxis={false} showXAxis={false} className="h-24 w-full" />
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