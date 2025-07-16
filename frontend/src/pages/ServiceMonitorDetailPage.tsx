import React, { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getMonitorById } from '../services/serviceMonitorService';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import HistoricalMonitorChart from '@/components/HistoricalMonitorChart';
import type { TimeRangeValue } from '@/components/TimeRangeSelector';
import { useTranslation } from 'react-i18next';

const ServiceMonitorDetailPage: React.FC = () => {
  const { t } = useTranslation();
  const { monitorId: monitorIdStr } = useParams<{ monitorId: string }>();
  const monitorId = parseInt(monitorIdStr || '0', 10);

  const [activeTab, setActiveTab] = useState<string>('realtime');

  const { data: monitor, isLoading, error } = useQuery({
    queryKey: ['monitor', monitorId],
    queryFn: () => getMonitorById(monitorId),
    enabled: !!monitorId,
  });

  if (isLoading) {
    return (
      <div className="p-4 space-y-4">
        <Skeleton className="h-8 w-1/2" />
        <Skeleton className="h-6 w-3/4" />
        <Skeleton className="h-6 w-3/4" />
        <Card className="flex-grow flex flex-col">
          <CardHeader>
            <Skeleton className="h-8 w-1/4" />
          </CardHeader>
          <CardContent>
            <Skeleton className="h-96 w-full" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (error) {
    return <div className="p-4 text-destructive">{t('vpsDetailPage.errors.loadMonitorData')}</div>;
  }

  if (!monitor) {
    return <div className="p-4">{t('vpsDetailPage.serviceMonitoring.monitorNotFound')}</div>;
  }

  return (
    <div className="flex flex-col h-full p-4 md:p-6 lg:p-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold mb-2">{monitor.name}</h1>
        <p className="text-muted-foreground">
          <span className="font-semibold">{t('vpsDetailPage.serviceMonitoring.type')}:</span> {monitor.monitorType}
        </p>
        <p className="text-muted-foreground">
          <span className="font-semibold">{t('vpsDetailPage.serviceMonitoring.target')}:</span> {monitor.target}
        </p>
      </div>

      <Card className="flex-grow flex flex-col">
        <CardHeader>
          <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
            <CardTitle>{t('vpsDetailPage.performanceMetrics.title')}</CardTitle>
            <Tabs value={activeTab} onValueChange={setActiveTab}>
              <TabsList>
                <TabsTrigger value="realtime">{t('vpsDetailPage.tabs.realtime')}</TabsTrigger>
                <TabsTrigger value="1h">{t('vpsDetailPage.tabs.1h')}</TabsTrigger>
                <TabsTrigger value="6h">{t('vpsDetailPage.tabs.6h')}</TabsTrigger>
                <TabsTrigger value="1d">{t('vpsDetailPage.tabs.1d')}</TabsTrigger>
                <TabsTrigger value="7d">{t('vpsDetailPage.tabs.7d')}</TabsTrigger>
              </TabsList>
            </Tabs>
          </div>
        </CardHeader>
        <CardContent className="flex-grow min-h-0">
          <HistoricalMonitorChart
            monitorId={monitorId}
            viewMode={activeTab === 'realtime' ? 'realtime' : 'historical'}
            timeRange={activeTab as TimeRangeValue}
          />
        </CardContent>
      </Card>
    </div>
  );
};

export default ServiceMonitorDetailPage;