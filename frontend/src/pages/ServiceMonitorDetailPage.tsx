import React, { useState, useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getMonitorById } from '../services/serviceMonitorService';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import HistoricalMonitorChart from '@/components/HistoricalMonitorChart';
import { TimeRangeSelector, getTimeRangeDetails, type TimeRangeValue } from '@/components/TimeRangeSelector';

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId: monitorIdStr } = useParams<{ monitorId: string }>();
  const monitorId = parseInt(monitorIdStr || '0', 10);

  const [timeRange, setTimeRange] = useState<TimeRangeValue>('1h');

  const selectedTimeRangeDetails = useMemo(() => {
    return getTimeRangeDetails(timeRange);
  }, [timeRange]);

  const { data: monitor, isLoading, error } = useQuery({
    queryKey: ['monitor', monitorId],
    queryFn: () => getMonitorById(monitorId),
    enabled: !!monitorId,
  });

  if (isLoading) {
    return (
      <div className="p-4">
        <Skeleton className="h-10 w-1/2 mb-4" />
        <Skeleton className="h-96 w-full" />
      </div>
    );
  }

  if (error) {
    return <div className="p-4 text-destructive">Error loading monitor details.</div>;
  }

  if (!monitor) {
    return <div className="p-4">Monitor not found.</div>;
  }

  return (
    <div className="flex flex-col h-full p-4">
      <h1 className="text-2xl font-bold mb-4">{monitor.name}</h1>
      <p className="mb-2"><span className="font-semibold">Type:</span> {monitor.monitorType}</p>
      <p className="mb-4"><span className="font-semibold">Target:</span> {monitor.target}</p>

      <Card className="flex-grow flex flex-col">
        <CardHeader>
          <div className="flex justify-between items-center">
            <CardTitle>Historical Performance</CardTitle>
            <TimeRangeSelector value={timeRange} onValueChange={setTimeRange} />
          </div>
        </CardHeader>
        <CardContent className="flex-grow">
          <HistoricalMonitorChart
            monitorId={monitorId}
            startTime={selectedTimeRangeDetails.startTime}
            endTime={selectedTimeRangeDetails.endTime}
            interval={selectedTimeRangeDetails.interval}
          />
        </CardContent>
      </Card>
    </div>
  );
};

export default ServiceMonitorDetailPage;