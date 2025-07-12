import React, { useEffect, useState, useMemo } from 'react';
import { useServerListStore } from '../store/serverListStore';
import type { ServiceMonitorResult } from '../types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, ReferenceArea } from 'recharts';
import {
  ChartContainer,
  ChartTooltip,
  ChartLegend,
  type ChartConfig,
} from "@/components/ui/chart";

type ChartPoint = { time: string; [key: string]: number | null | string };

const formatDateTick = (tickItem: string) => new Date(tickItem).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
const formatTooltipLabel = (label: string) => new Date(label).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

interface RealtimeMonitorChartProps {
  monitorId: number;
}

const RealtimeMonitorChart: React.FC<RealtimeMonitorChartProps> = ({ monitorId }) => {
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isChartLoading, setIsChartLoading] = useState(true);
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});
  // By using selectors, we prevent re-renders when other parts of the store change.
  const getInitialMonitorResults = useServerListStore(state => state.getInitialMonitorResults);
  const subscribeToMonitorResults = useServerListStore(state => state.subscribeToMonitorResults);

  useEffect(() => {
    
    const fetchInitialData = async () => {
      setIsChartLoading(true);
      try {
        const initialResults = await getInitialMonitorResults(monitorId, 500);
        setResults(initialResults);
      } catch (error) {
        console.error("Failed to fetch initial monitor results:", error);
      } finally {
        setIsChartLoading(false);
      }
    };

    fetchInitialData();

    const unsubscribe = subscribeToMonitorResults(monitorId, (newResults) => {
      // The problematic 'if' condition is removed. We now directly update the state.
      setResults(prevResults => {
        const updated = [...prevResults, ...newResults];
        const MAX_POINTS = 1000;
        if (updated.length > MAX_POINTS) {
          return updated.slice(updated.length - MAX_POINTS);
        }
        return updated;
      });
    });

    return () => {
      unsubscribe();
    };
  }, [monitorId, getInitialMonitorResults, subscribeToMonitorResults]);

  const agentConfig = useMemo(() => {
    if (!results || results.length === 0) {
      return { agentLines: [], chartConfig: {} as ChartConfig };
    }
    const groupedByAgent = results.reduce((acc, result) => {
      if (!acc[result.agentName]) acc[result.agentName] = [];
      acc[result.agentName].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    // Sort agent names alphabetically to ensure a stable color assignment.
    const sortedAgentNames = Object.keys(groupedByAgent).sort();

    const agentLines = sortedAgentNames.map((agentName, index) => ({
      dataKey: agentName,
      name: agentName,
      stroke: AGENT_COLORS[index % AGENT_COLORS.length],
    }));

    const chartConfig = agentLines.reduce((acc, line) => {
      acc[line.dataKey] = { label: line.name, color: line.stroke };
      return acc;
    }, {} as ChartConfig);

    return { agentLines, chartConfig };
  }, [results]);

  const chartData = useMemo(() => {
    if (!results || results.length === 0) {
      return { chartData: [], downtimeAreas: [] as { x1: string, x2: string }[] };
    }

    const groupedByTime = results.reduce((acc, result) => {
        const time = result.time;
        if (!acc[time]) {
            acc[time] = { time };
        }
        acc[time][result.agentName] = result.isUp ? result.latencyMs : null;
        return acc;
    }, {} as Record<string, ChartPoint>);

    const finalChartData = Object.values(groupedByTime).sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
    return { chartData: finalChartData, downtimeAreas: [] };
  }, [results]);

  const handleLegendClick = (data: { dataKey: string }) => {
    const dataKey = data.dataKey as string;
    setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
  };

  if (isChartLoading) {
    return (
      <div className="absolute inset-0 flex items-center justify-center bg-background/50 z-10">
        <div className="text-center">
          <p className="text-lg font-semibold">正在加载图表...</p>
          <p className="text-sm text-muted-foreground">请稍候</p>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex-grow min-h-0 ${isChartLoading ? 'opacity-50' : ''}`}>
      {chartData.chartData.length > 0 ? (
        <ChartContainer config={agentConfig.chartConfig} className="h-full w-full">
          <LineChart data={chartData.chartData} margin={{ top: 5, right: 20, left: 20, bottom: 5 }}>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="time"
              tickFormatter={formatDateTick}
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              tick={{ fontSize: 11 }}
              type="category"
              allowDuplicatedCategory={false}
            />
            <YAxis
              width={80}
              tickFormatter={(tick) => `${tick}ms`}
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              label={{ value: '延迟 (ms)', angle: -90, position: 'insideLeft', offset: -15 }}
            />
            <ChartTooltip
              cursor={true}
              content={({ active, payload, label }) => {
                if (active && payload && payload.length && label) {
                  const labelStr = formatTooltipLabel(label);
                  return (
                    <div className="z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 text-sm shadow-md animate-in fade-in-0 zoom-in-95">
                      <div className="font-semibold">{labelStr}</div>
                      <div className="mt-2 space-y-2">
                        {payload.map((item) => {
                          const key = item.dataKey as string;
                          const config = agentConfig.chartConfig[key as keyof typeof agentConfig.chartConfig];
                          const color = config?.color || item.color;
                          
                          if (item.value === null || item.value === undefined) return null;
                          
                          const value = typeof item.value === 'number' ? `${Math.round(item.value as number)} ms` : 'N/A';

                          return (
                            <div key={item.dataKey} className="flex items-center gap-2">
                              <div
                                className="h-2.5 w-2.5 shrink-0 rounded-[2px]"
                                style={{ backgroundColor: color }}
                              />
                              <div className="flex flex-1 justify-between">
                                <span className="text-muted-foreground">{config?.label || item.name}:</span>
                                <span className="font-mono font-medium">{value}</span>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  );
                }
                return null;
              }}
            />
            <ChartLegend
              content={({ payload }) => (
                <div className="flex items-center justify-center gap-4 pt-3">
                  {payload?.map((item) => {
                    const key = item.dataKey as string;
                    const itemConfig = agentConfig.chartConfig[key as keyof typeof agentConfig.chartConfig];
                    const isHidden = hiddenLines[key];
                    return (
                      <div
                        key={item.value}
                        onClick={() => {
                          if (typeof item.dataKey === 'string') {
                            handleLegendClick({ dataKey: item.dataKey });
                          }
                        }}
                        className="flex items-center gap-1.5 cursor-pointer"
                      >
                        <div
                          className="h-2 w-2 shrink-0 rounded-[2px]"
                          style={{ backgroundColor: isHidden ? '#A0A0A0' : item.color }}
                        />
                        <span style={{ color: isHidden ? '#A0A0A0' : 'inherit' }}>
                          {itemConfig?.label || item.value}
                        </span>
                      </div>
                    );
                  })}
                </div>
              )}
            />
            {chartData.downtimeAreas.map((area, index) => (
              <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="hsl(var(--destructive))" fillOpacity={0.15} ifOverflow="extendDomain" />
            ))}
            {agentConfig.agentLines.map(line => (
              <Line
                key={line.dataKey}
                type="monotone"
                dataKey={line.dataKey}
                name={line.name}
                stroke={`var(--color-${line.dataKey})`}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 6 }}
                connectNulls={true}
                hide={hiddenLines[line.dataKey]}
                isAnimationActive={false}
              />
            ))}
          </LineChart>
        </ChartContainer>
      ) : (
        <div className="flex items-center justify-center h-full">
          <p className="text-muted-foreground">暂无实时监控结果。</p>
        </div>
      )}
    </div>
  );
};

export default RealtimeMonitorChart;