import React, { useEffect, useState, useMemo, useRef } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getMonitorById, getMonitorResults } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorResult } from '../types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, ReferenceArea } from 'recharts';
import websocketService from '../services/websocketService';
import { ArrowLeft, CheckCircle, XCircle } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  ChartContainer,
  ChartTooltip,
  ChartLegend,
  type ChartConfig,
} from "@/components/ui/chart";

const TIME_RANGE_OPTIONS = [
  { label: '实时', value: 'realtime' as const },
  { label: '1H', value: '1h' as const },
  { label: '6H', value: '6h' as const },
  { label: '24H', value: '24h' as const },
  { label: '7D', value: '7d' as const },
];
type TimeRangeOption = typeof TIME_RANGE_OPTIONS[number]['value'];

type ChartPoint = { time: string; [key: string]: number | null | string };

const formatDateTick = (tickItem: string) => new Date(tickItem).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
const formatTooltipLabel = (label: string) => new Date(label).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];
const MAX_HISTORICAL_POINTS = 300;
const REALTIME_UPDATE_INTERVAL = 2000; // 2 seconds

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});
  
  const newResultsBuffer = useRef<ServiceMonitorResult[]>([]);

  const timeRangeToMillis = useMemo(() => ({ '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 }), []);

  const agentConfig = useMemo(() => {
    if (!results || results.length === 0) {
      return { agentLines: [], chartConfig: {} as ChartConfig };
    }
    const groupedByAgent = results.reduce((acc, result) => {
      if (!acc[result.agentName]) acc[result.agentName] = [];
      acc[result.agentName].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    const agentLines = Object.keys(groupedByAgent).map((agentName, index) => ({
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


  useEffect(() => {
    if (!monitorId) return;

    const fetchInitialData = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(parseInt(monitorId, 10));
        setMonitor(monitorData);

        let resultsData: ServiceMonitorResult[];
        if (selectedTimeRange === 'realtime') {
          resultsData = await getMonitorResults(parseInt(monitorId, 10), undefined, undefined, 500); // Fetch more for interpolation buffer
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange as Exclude<TimeRangeOption, 'realtime'>]);
          resultsData = await getMonitorResults(parseInt(monitorId, 10), startTime.toISOString(), endTime.toISOString());
        }
        setResults(resultsData.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime()));
        setError(null);
      } catch (err) {
        setError('无法获取监控详情。');
        console.error(err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchInitialData();
  }, [monitorId, selectedTimeRange, timeRangeToMillis]);

  useEffect(() => {
    if (!monitorId || selectedTimeRange !== 'realtime') return;

    const handleNewResult = (result: ServiceMonitorResult) => {
      if (result.monitorId !== parseInt(monitorId, 10)) return;
      newResultsBuffer.current.push(result);
    };

    websocketService.on('service_monitor_result', handleNewResult);
    return () => {
      websocketService.off('service_monitor_result', handleNewResult);
    };
  }, [monitorId, selectedTimeRange]);

  useEffect(() => {
    if (selectedTimeRange !== 'realtime') return;

    const intervalId = setInterval(() => {
      if (newResultsBuffer.current.length > 0) {
        setResults(prevResults => {
          const updatedResults = [...prevResults, ...newResultsBuffer.current]
            .sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
          newResultsBuffer.current = [];
          // Keep a reasonable buffer size
          return updatedResults.length > 1000 ? updatedResults.slice(updatedResults.length - 1000) : updatedResults;
        });
      }
    }, REALTIME_UPDATE_INTERVAL);

    return () => clearInterval(intervalId);
  }, [selectedTimeRange]);

  const chartData = useMemo(() => {
    if (!results || results.length === 0) {
      return { chartData: [], downtimeAreas: [] };
    }
    
    const groupedByAgent = results.reduce((acc, result) => {
      if (!acc[result.agentName]) acc[result.agentName] = [];
      acc[result.agentName].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    const latestTime = new Date(Math.max(...results.map(r => new Date(r.time).getTime())));
    let startTime;
    let step;

    if (selectedTimeRange === 'realtime') {
      const earliestTime = new Date(Math.min(...results.map(r => new Date(r.time).getTime())));
      const timeDiff = latestTime.getTime() - earliestTime.getTime();
      startTime = earliestTime;
      step = Math.max(1000, timeDiff / MAX_HISTORICAL_POINTS); // Dynamic step for realtime
    } else {
      startTime = new Date(latestTime.getTime() - timeRangeToMillis[selectedTimeRange]);
      const timeDiff = latestTime.getTime() - startTime.getTime();
      step = Math.max(15 * 1000, timeDiff / MAX_HISTORICAL_POINTS);
    }

    const timePoints = [];
    for (let t = startTime.getTime(); t <= latestTime.getTime(); t += step) {
      timePoints.push(t);
    }

    const data = timePoints.map(timeNum => {
      const time = new Date(timeNum).toISOString();
      const point: ChartPoint = { time };

      for (const agentName in groupedByAgent) {
        const agentResults = groupedByAgent[agentName].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
        const resultIndex = agentResults.findIndex(r => new Date(r.time).getTime() >= timeNum);

        if (resultIndex === -1 || resultIndex === 0) {
          point[agentName] = null;
        } else {
          const p1 = agentResults[resultIndex - 1];
          const p2 = agentResults[resultIndex];
          if (!p1.isUp || !p2.isUp || typeof p1.latencyMs !== 'number' || typeof p2.latencyMs !== 'number') {
            point[agentName] = null;
          } else {
            const t1 = new Date(p1.time).getTime();
            const t2 = new Date(p2.time).getTime();
            const v1 = p1.latencyMs;
            const v2 = p2.latencyMs;
            point[agentName] = v1 + (v2 - v1) * ((timeNum - t1) / (t2 - t1));
          }
        }
      }
      return point;
    });

    const areas: { x1: string, x2: string }[] = [];
    // Downtime calculation can be added here if needed
    return { chartData: data, downtimeAreas: areas };
  }, [results, selectedTimeRange, timeRangeToMillis]);

  const handleLegendClick = (data: { dataKey: string }) => {
    const dataKey = data.dataKey as string;
    setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
  };

  if (isLoading) return <div className="container mx-auto p-8 text-center">正在加载...</div>;
  if (error) return (
    <div className="container mx-auto p-8">
      <Alert variant="destructive">
        <AlertTitle>错误</AlertTitle>
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    </div>
  );
  if (!monitor) return <div className="container mx-auto p-8 text-center">未找到监控项。</div>;

  return (
    <div className="container mx-auto p-4 md:p-6 lg:p-8 space-y-6">
      <Card>
        <CardHeader className="flex flex-col sm:flex-row justify-between items-start gap-4">
          <div>
            <CardTitle className="text-3xl font-bold">{monitor.name}</CardTitle>
            <CardDescription className="text-lg mt-1">{monitor.monitorType.toUpperCase()} - {monitor.target}</CardDescription>
          </div>
          <Button asChild variant="outline">
            <Link to="/monitors"><ArrowLeft className="w-4 h-4 mr-2" /> 返回列表</Link>
          </Button>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-center">
            <Card>
              <CardHeader><CardTitle className="text-sm font-medium text-muted-foreground">检查频率</CardTitle></CardHeader>
              <CardContent><p className="text-2xl font-semibold">{monitor.frequencySeconds}s</p></CardContent>
            </Card>
            <Card>
              <CardHeader><CardTitle className="text-sm font-medium text-muted-foreground">超时</CardTitle></CardHeader>
              <CardContent><p className="text-2xl font-semibold">{monitor.timeoutSeconds}s</p></CardContent>
            </Card>
            <Card>
              <CardHeader><CardTitle className="text-sm font-medium text-muted-foreground">状态</CardTitle></CardHeader>
              <CardContent>
                <Badge variant={monitor.isActive ? 'success' : 'destructive'} className="text-lg">
                  {monitor.isActive ? <CheckCircle className="w-4 h-4 mr-2" /> : <XCircle className="w-4 h-4 mr-2" />}
                  {monitor.isActive ? 'Active' : 'Inactive'}
                </Badge>
              </CardContent>
            </Card>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex flex-col sm:flex-row justify-between items-center gap-4">
            <CardTitle>监控结果</CardTitle>
            <ToggleGroup type="single" value={selectedTimeRange} onValueChange={(value) => value && setSelectedTimeRange(value as TimeRangeOption)} aria-label="Time range">
              {TIME_RANGE_OPTIONS.map(period => (
                <ToggleGroupItem key={period.value} value={period.value}>{period.label}</ToggleGroupItem>
              ))}
            </ToggleGroup>
          </div>
        </CardHeader>
        <CardContent className="h-96">
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
                    isAnimationActive={true}
                  />
                ))}
              </LineChart>
            </ChartContainer>
          ) : (
            <div className="flex items-center justify-center h-full">
              <p className="text-muted-foreground">暂无监控结果。</p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
};

export default ServiceMonitorDetailPage;