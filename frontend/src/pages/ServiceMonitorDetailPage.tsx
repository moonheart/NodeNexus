import React, { useEffect, useState, useMemo } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getMonitorById, getMonitorResults } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorResult } from '../types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea, type LegendProps } from 'recharts';
import websocketService from '../services/websocketService';
import { ArrowLeft, CheckCircle, XCircle } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import type { ValueType } from 'recharts/types/component/DefaultTooltipContent';

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
const formatLatencyForTooltip = (value: ValueType) => typeof value === 'number' ? `${value.toFixed(0)} ms` : `${value}`;

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');
  const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});

  const timeRangeToMillis: Record<Exclude<TimeRangeOption, 'realtime'>, number> = { '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 };

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

  const { chartData, agentLines, downtimeAreas } = useMemo(() => {
    if (!results || results.length === 0) {
      return { chartData: [], agentLines: [], downtimeAreas: [] };
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

    const timePoints = [...new Set(results.map(r => new Date(r.time).toISOString()))].sort();
    const chartData = timePoints.map(time => {
      const point: { time: string; [key: string]: number | null | string } = { time };
      for (const agentName in groupedByAgent) {
        const resultForTime = groupedByAgent[agentName].find(r => new Date(r.time).toISOString() === time);
        point[agentName] = resultForTime && resultForTime.isUp ? resultForTime.latencyMs : null;
      }
      return point;
    });

    const areas: { x1: string, x2: string }[] = [];
    let downtimeStart: string | null = null;
    for (let i = 0; i < timePoints.length; i++) {
      const time = timePoints[i];
      const isDown = Object.values(groupedByAgent).some(agentResults => agentResults.some(r => new Date(r.time).toISOString() === time && !r.isUp));
      if (isDown && !downtimeStart) {
        downtimeStart = time;
      } else if (!isDown && downtimeStart) {
        const prevTime = i > 0 ? timePoints[i - 1] : downtimeStart;
        areas.push({ x1: downtimeStart, x2: prevTime });
        downtimeStart = null;
      }
    }
    if (downtimeStart) {
      areas.push({ x1: downtimeStart, x2: timePoints[timePoints.length - 1] });
    }

    return { chartData, agentLines, downtimeAreas: areas };
  }, [results]);

  const handleLegendClick: LegendProps['onClick'] = (data) => {
    const dataKey = data.dataKey as string;
    setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
  };

  const renderLegendText: LegendProps['formatter'] = (value, entry) => {
    const { color, dataKey } = entry;
    const isHidden = typeof dataKey === 'string' && hiddenLines[dataKey];
    return <span style={{ color: isHidden ? '#A0A0A0' : color || '#000', cursor: 'pointer' }}>{value}</span>;
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
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
                <YAxis label={{ value: '延迟 (ms)', angle: -90, position: 'insideLeft' }} tickFormatter={(tick) => `${tick}ms`} />
                <Tooltip formatter={formatLatencyForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
                <Legend onClick={handleLegendClick} formatter={renderLegendText} />
                {downtimeAreas.map((area, index) => (
                  <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="hsl(var(--destructive))" fillOpacity={0.15} />
                ))}
                {agentLines.map(line => (
                  <Line
                    key={line.dataKey}
                    type="monotone"
                    dataKey={line.dataKey}
                    name={line.name}
                    stroke={hiddenLines[line.dataKey] ? 'transparent' : line.stroke}
                    dot={false}
                    activeDot={{ r: 6 }}
                    connectNulls={true}
                    strokeWidth={2}
                  />
                ))}
              </LineChart>
            </ResponsiveContainer>
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