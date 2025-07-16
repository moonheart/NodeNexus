import React from 'react';
import { useTranslation } from 'react-i18next';
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import UnifiedMetricChart from './metric/UnifiedMetricChart';

interface VpsMetricsChartProps {
  vpsId: number;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

/**
 * A component that displays metric charts for a VPS in a tabbed view.
 * Used in VPS list cards for a quick overview.
 * This component has been refactored to use the UnifiedMetricChart.
 */
export const VpsMetricsChart: React.FC<VpsMetricsChartProps> = ({ vpsId, activeTab, onTabChange }) => {
  const { t } = useTranslation();

  const commonChartProps = {
    sourceType: 'vps' as const,
    sourceId: vpsId,
    viewMode: 'realtime' as const,
    showTitle: false,
    showYAxis: false,
    showXAxis: false,
    showLegend: false,
    className: "h-24 w-full",
  };

  return (
    <Tabs value={activeTab} onValueChange={onTabChange} className="w-full">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="service">{t('vps.serviceMonitor')}</TabsTrigger>
        <TabsTrigger value="cpu">{t('vps.cpu')}</TabsTrigger>
        <TabsTrigger value="ram">{t('vps.ram')}</TabsTrigger>
      </TabsList>
      <TabsContent value="service">
        <UnifiedMetricChart {...commonChartProps} metricType="service-latency" />
      </TabsContent>
      <TabsContent value="cpu">
        <UnifiedMetricChart {...commonChartProps} metricType="cpu" />
      </TabsContent>
      <TabsContent value="ram">
        <UnifiedMetricChart {...commonChartProps} metricType="ram" />
      </TabsContent>
    </Tabs>
  );
};