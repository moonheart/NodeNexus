import React, { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useMetrics, type ChartDataPoint, type ChartSourceType, type ChartViewMode } from '@/hooks/useMetrics';
import { getChartConfig, type MetricType } from '@/utils/chartConfigFactory';
import type { TimeRangeValue } from '@/components/TimeRangeSelector';
import ServerMetricsChart from './ServerMetricsChart';

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];
const REALTIME_WINDOW_SECONDS = 5 * 60; // 5 minutes

export interface UnifiedMetricChartProps {
  sourceType: ChartSourceType;
  sourceId: number;
  metricType: MetricType;
  viewMode: ChartViewMode;
  timeRange?: TimeRangeValue;
  
  // Optional data injection
  data?: ChartDataPoint[];
  loading?: boolean;

  // UI options
  showTitle?: boolean;
  showYAxis?: boolean;
  showXAxis?: boolean;
  showLegend?: boolean;
  className?: string;
}

const UnifiedMetricChart: React.FC<UnifiedMetricChartProps> = ({
  sourceType,
  sourceId,
  metricType,
  viewMode,
  timeRange,
  data: injectedData,
  loading: injectedLoading,
  showTitle = true,
  showYAxis = true,
  showXAxis = true,
  showLegend,
  className,
}) => {
  const { t } = useTranslation();

  // Use injected data if provided, otherwise fetch it.
  const hookResult = useMetrics({
    sourceType,
    sourceId,
    metricType,
    viewMode,
    timeRange,
    preserveDataOnFetch: true,
    enabled: injectedData === undefined,
  });

  const { data, loading, error, ramTotal } = useMemo(() => {
    if (injectedData !== undefined) {
      return {
        data: injectedData,
        loading: injectedLoading ?? false,
        error: null,
        ramTotal: undefined // ramTotal is not available with injected data
      };
    }
    return hookResult;
  }, [injectedData, injectedLoading, hookResult]);

  const chartConfig = useMemo(
    () => getChartConfig({ metricType, t, ramTotal }),
    [metricType, t, ramTotal]
  );

  // Dynamically generate lines for service latency charts
  const finalLines = useMemo(() => {
    if (metricType === 'service-latency' && data.length > 0) {
      const dataKeys = new Set<string>();
      data.forEach(point => {
        Object.keys(point).forEach(key => {
          if (key !== 'time') {
            dataKeys.add(key);
          }
        });
      });
      
      return Array.from(dataKeys).map((key, index) => ({
        dataKey: key,
        name: key,
        stroke: AGENT_COLORS[index % AGENT_COLORS.length],
      }));
    }
    return chartConfig.lines;
  }, [metricType, data, chartConfig.lines]);

  // --- Formatters and Domains based on viewMode ---
  const now = Date.now();
  const realtimeXAxisDomain: [number, number] = [now - REALTIME_WINDOW_SECONDS * 1000, now];

  const xAxisDomain = viewMode === 'realtime' ? realtimeXAxisDomain : undefined;
  
  const xAxisFormatter = (tick: number) => {
    if (viewMode === 'realtime') {
      return new Date(tick).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
    }
    return new Date(tick).toLocaleString();
  };

  const tooltipLabelFormatter = (label: number) => {
     if (viewMode === 'realtime') {
        return new Date(label).toLocaleTimeString();
     }
     return new Date(label).toLocaleString();
  };

  const noDataMessage = viewMode === 'realtime' 
    ? t('components.realtimeMetricChart.noRealtimeData') 
    : t('vps.noData');

  return (
    <div className={`flex w-full flex-col ${className}`}>
      {showTitle && <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{chartConfig.title}</h3>}
      <div className="relative flex-grow">
        <ServerMetricsChart
          data={data}
          loading={loading}
          error={error}
          lines={finalLines}
          chartType={chartConfig.chartType}
          showLegend={showLegend ?? chartConfig.showLegend}
          showYAxis={showYAxis}
          showXAxis={showXAxis}
          yAxisDomain={chartConfig.yAxisDomain}
          yAxisFormatter={chartConfig.yAxisFormatter}
          xAxisDomain={xAxisDomain}
          xAxisFormatter={xAxisFormatter}
          tooltipLabelFormatter={tooltipLabelFormatter}
          tooltipValueFormatter={chartConfig.tooltipValueFormatter}
          noDataMessage={noDataMessage}
        />
      </div>
    </div>
  );
};

export default UnifiedMetricChart;