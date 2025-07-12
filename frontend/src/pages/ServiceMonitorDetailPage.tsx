import React, { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getMonitorById } from '../services/serviceMonitorService';
import type { ServiceMonitor } from '../types';
import { ArrowLeft, CheckCircle, XCircle } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import RealtimeMonitorChart from '@/components/RealtimeMonitorChart';
import HistoricalMonitorChart from '@/components/HistoricalMonitorChart';

const TIME_RANGE_OPTIONS = [
  { label: '实时', value: 'realtime' as const },
  { label: '1H', value: '1h' as const },
  { label: '6H', value: '6h' as const },
  { label: '24H', value: '24h' as const },
  { label: '7D', value: '7d' as const },
];
type TimeRangeOption = typeof TIME_RANGE_OPTIONS[number]['value'];

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');


  // Effect to fetch monitor details
  useEffect(() => {
    if (!monitorId) return;
    const monitorIdNum = parseInt(monitorId, 10);

    const fetchMonitorDetails = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(monitorIdNum);
        setMonitor(monitorData);
      } catch (err) {
        setError('无法获取监控详情。');
        console.error(err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchMonitorDetails();
  }, [monitorId]);


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
        <CardContent className="h-96 relative flex flex-col">
          {monitorId && (
            selectedTimeRange === 'realtime' ? (
              <RealtimeMonitorChart monitorId={parseInt(monitorId, 10)} />
            ) : (
              <HistoricalMonitorChart
                monitorId={parseInt(monitorId, 10)}
                timeRange={selectedTimeRange as Exclude<TimeRangeOption, 'realtime'>}
              />
            )
          )}
        </CardContent>
      </Card>
    </div>
  );
};

export default ServiceMonitorDetailPage;