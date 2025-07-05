import React, { useEffect, useState, useMemo } from 'react';
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
  ChartTooltipContent,
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

const formatDateTick = (tickItem: string) => new Date(tickItem).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
const formatTooltipLabel = (label: string) => new Date(label).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});

  const timeRangeToMillis = useMemo(() => ({ '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 }), []);

  useEffect(() => {
    if (!monitorId) return;

    const fetchInitialData = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(parseInt(monitorId, 10));
        setMonitor(monitorData);

        let resultsData: ServiceMonitorResult[];
        if (selectedTimeRange === 'realtime') {
          resultsData = await getMonitorResults(parseInt(monitorId, 10), undefined, undefined, 300);
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
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
  }, [monitorId, selectedTimeRange]);

  useEffect(() => {
    if (!monitorId || selectedTimeRange !== 'realtime') return;

    const handleNewResult = (result: ServiceMonitorResult) => {
      if (result.monitorId !== parseInt(monitorId, 10)) return;
      setResults(prevResults => {
        const updatedResults = [...prevResults, result].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
        return updatedResults.length > 300 ? updatedResults.slice(updatedResults.length - 300) : updatedResults;
      });
    };

    websocketService.on('service_monitor_result', handleNewResult);
    return () => {
      websocketService.off('service_monitor_result', handleNewResult);
    };
  }, [monitorId, selectedTimeRange]);

  const { chartData, agentLines, downtimeAreas, chartConfig } = useMemo(() => {
    if (!results || results.length === 0) {
      return { chartData: [], agentLines: [], downtimeAreas: [], chartConfig: {} as ChartConfig };
    }

    const groupedByAgent = results.reduce((acc, result) => {
      const agentName = result.agentName;
      if (!acc[agentName]) acc[agentName] = [];
      acc[agentName].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    const agentLines = Object.keys(groupedByAgent).map((agentName, index) => ({
      dataKey: agentName,
      name: agentName,
      stroke: AGENT_COLORS[index % AGENT_COLORS.length],
    }));

    const chartConfig = agentLines.reduce((acc, line) => {
      acc[line.dataKey] = {
        label: line.name,
        color: line.stroke,
      };
      return acc;
    }, {} as ChartConfig);

    const endTime = results.length > 0 ? new Date(Math.max(...results.map(r => new Date(r.time).getTime()))) : new Date();
    let startTime = new Date(endTime);
    let step = 15 * 1000; // 15 seconds for realtime

    if (selectedTimeRange !== 'realtime') {
      startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
      const timeDiff = endTime.getTime() - startTime.getTime();
      step = Math.max(15 * 1000, timeDiff / 300); // at most 300 points
    } else {
      const earliestTime = results.length > 0 ? new Date(Math.min(...results.map(r => new Date(r.time).getTime()))) : new Date();
      startTime = earliestTime;
    }

    const timePoints = [];
    for (let t = startTime.getTime(); t <= endTime.getTime(); t += step) {
      timePoints.push(t);
    }

    const chartData = timePoints.map(timeNum => {
      const time = new Date(timeNum).toISOString();
      const point: { time: string; [key: string]: number | null | string } = { time };

      for (const agentName in groupedByAgent) {
        const agentResults = groupedByAgent[agentName].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
        const resultIndex = agentResults.findIndex(r => new Date(r.time).getTime() >= timeNum);

        if (resultIndex === -1) {
          point[agentName] = null;
        } else if (resultIndex === 0) {
          point[agentName] = agentResults[0].isUp ? agentResults[0].latencyMs : null;
        } else {
          const p1 = agentResults[resultIndex - 1];
          const p2 = agentResults[resultIndex];
          const t1 = new Date(p1.time).getTime();
          const t2 = new Date(p2.time).getTime();

          if (!p1.isUp || !p2.isUp) {
            point[agentName] = null;
          } else {
            const v1 = p1.latencyMs;
            const v2 = p2.latencyMs;
            if (typeof v1 === 'number' && typeof v2 === 'number') {
              const interpolatedValue = v1 + (v2 - v1) * ((timeNum - t1) / (t2 - t1));
              point[agentName] = interpolatedValue;
            } else {
              point[agentName] = null;
            }
          }
        }
      }
      return point;
    });

    const areas: { x1: string, x2: string }[] = [];
    let downtimeStart: number | null = null;
    for (let i = 0; i < timePoints.length; i++) {
      const time = timePoints[i];
      const isDown = Object.values(groupedByAgent).some(agentResults => {
        const resultIndex = agentResults.findIndex(r => new Date(r.time).getTime() >= time);
        if (resultIndex === -1 || resultIndex === 0) return false;
        const p1 = agentResults[resultIndex - 1];
        return !p1.isUp;
      });

      if (isDown && downtimeStart === null) {
        downtimeStart = time;
      } else if (!isDown && downtimeStart !== null) {
        const prevTime = i > 0 ? timePoints[i - 1] : downtimeStart;
        areas.push({ x1: new Date(downtimeStart).toISOString(), x2: new Date(prevTime).toISOString() });
        downtimeStart = null;
      }
    }
    if (downtimeStart !== null) {
      areas.push({ x1: new Date(downtimeStart).toISOString(), x2: new Date(timePoints[timePoints.length - 1]).toISOString() });
    }

    return { chartData, agentLines, downtimeAreas: areas, chartConfig };
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
          {results.length > 0 ? (
            <ChartContainer config={chartConfig} className="h-full w-full">
              <LineChart data={chartData} margin={{ top: 5, right: 20, left: 20, bottom: 5 }}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="time"
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
                  content={
                    <ChartTooltipContent
                      labelFormatter={formatTooltipLabel}
                      formatter={(value, name, item) => {
                        if (value == null) return null;
                        const itemConfig = chartConfig[name as keyof typeof chartConfig];
                        return (
                          <div className="flex w-full items-center gap-2">
                            <div
                              className="shrink-0 rounded-[2px] h-2.5 w-2.5"
                              style={{ backgroundColor: item.color }}
                            />
                            <div className="flex flex-1 justify-between leading-none">
                              <span className="text-muted-foreground">
                                {itemConfig?.label || name}
                              </span>
                              <span className="text-foreground font-mono font-medium tabular-nums">
                                {`${Math.round(value as number)} ms`}
                              </span>
                            </div>
                          </div>
                        );
                      }}
                      indicator="dot"
                    />
                  }
                />
                <ChartLegend
                  content={({ payload }) => (
                    <div className="flex items-center justify-center gap-4 pt-3">
                      {payload?.map((item) => {
                        const key = item.dataKey as string;
                        const itemConfig = chartConfig[key as keyof typeof chartConfig];
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
                {agentLines.map(line => (
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