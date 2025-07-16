import React, { useState, useMemo } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, ReferenceArea } from 'recharts';
import {
  ChartContainer,
  ChartTooltip,
  ChartLegend,
  type ChartConfig,
} from "@/components/ui/chart";
import { Skeleton } from './ui/skeleton';
import { useMetrics, type ChartViewMode } from '@/hooks/useMetrics';
import type { TimeRangeValue } from './TimeRangeSelector';

const formatDateTick = (tickItem: number) => new Date(tickItem).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
const formatTooltipLabel = (label: number) => new Date(label).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

interface HistoricalMonitorChartProps {
  monitorId: number;
  timeRange: TimeRangeValue;
  viewMode: ChartViewMode; // Kept for useMetrics, but animation is disabled
}

const HistoricalMonitorChart: React.FC<HistoricalMonitorChartProps> = ({ monitorId, timeRange, viewMode }) => {
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});
  
  const { data, loading: isChartLoading, error } = useMetrics({
    sourceType: 'monitor',
    sourceId: monitorId,
    metricType: 'service-latency',
    viewMode,
    timeRange,
  });

  const agentConfig = useMemo(() => {
    if (!data || data.length === 0) {
      return { agentLines: [], chartConfig: {} as ChartConfig };
    }
    const dataKeys = new Set<string>();
    data.forEach(point => {
      Object.keys(point).forEach(key => {
        if (key !== 'time') {
          dataKeys.add(key);
        }
      });
    });

    const sortedAgentNames = Array.from(dataKeys).sort();
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
  }, [data]);

  // TODO: Implement downtimeAreas calculation in useMetrics hook
  const downtimeAreas: { x1: number, x2: number }[] = [];

  const handleLegendClick = (data: { dataKey: string }) => {
    const { dataKey } = data;
    setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
  };

  if (isChartLoading) {
    return <Skeleton className="h-full w-full" />;
  }

  if (error) {
    return <div className="flex items-center justify-center h-full"><p className="text-destructive">{error}</p></div>;
  }

  return (
    <div className={`flex-grow min-h-0 ${isChartLoading ? 'opacity-50' : ''}`}>
      {data.length > 0 ? (
        <ChartContainer config={agentConfig.chartConfig} className="h-full w-full">
          <LineChart data={data} margin={{ top: 5, right: 20, left: 20, bottom: 5 }}>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="time"
              type="number"
              domain={['dataMin', 'dataMax']}
              tickFormatter={formatDateTick}
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              tick={{ fontSize: 11 }}
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
                  const labelStr = formatTooltipLabel(label as number);
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
            {downtimeAreas.map((area, index) => (
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
          <p className="text-muted-foreground">暂无历史监控结果。</p>
        </div>
      )}
    </div>
  );
};

export default HistoricalMonitorChart;