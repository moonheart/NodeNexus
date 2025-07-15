import React, { useEffect, useMemo } from 'react';
import { useServerListStore } from '../store/serverListStore';
import { useShallow } from 'zustand/react/shallow';
import { formatBytesForDisplay, formatNetworkSpeed } from '@/utils/vpsUtils';
import { useTranslation } from 'react-i18next';
import ServerMetricsChart from './ServerMetricsChart';

const REALTIME_WINDOW_SECONDS = 5 * 60; // 5 minutes

interface RealtimeMetricChartProps {
  vpsId: number;
  metricType: 'cpu' | 'ram' | 'network' | 'disk';
  showTitle?: boolean;
  showYAxis?: boolean;
  showXAxis?: boolean;
  className?: string;
}

const RealtimeMetricChart: React.FC<RealtimeMetricChartProps> = ({
  vpsId,
  metricType,
  showTitle = true,
  showYAxis = true,
  showXAxis = true,
  className,
}) => {
  const { t } = useTranslation();

  const { data, status } = useServerListStore(
    useShallow(state => state.initialVpsMetrics[vpsId] || { data: [], status: 'idle' })
  );

  const latestMetric = useServerListStore(
    useShallow(state => state.latestMetrics[vpsId])
  );

  useEffect(() => {
    useServerListStore.getState().ensureInitialVpsPerformanceMetrics(vpsId);
  }, [vpsId]);

  const now = Date.now();
  const timeDomain: [number, number] = [now - REALTIME_WINDOW_SECONDS * 1000, now];

  const ramTotal = latestMetric?.memoryTotalBytes ?? 0;

  const chartConfig = useMemo(() => {
    // Using hardcoded, reliable hex colors to ensure functionality.
    // These are chosen to be visually distinct and pleasant.
    const primaryColor = '#8884d8';
    const secondaryColor = '#82ca9d';

    switch (metricType) {
      case 'cpu':
        return {
          title: t('components.realtimeMetricChart.cpuUsageChartTitle'),
          lines: [{ dataKey: 'cpuUsagePercent', name: 'CPU', stroke: primaryColor }],
          yAxisDomain: [0, 100] as [number, number],
          yAxisFormatter: (value: number) => `${value}%`,
          tooltipValueFormatter: (value: number) => [`${value.toFixed(1)}%`, 'CPU'],
        };
      case 'ram':
        return {
          title: t('components.realtimeMetricChart.memoryUsageChartTitle'),
          lines: [{ dataKey: 'memoryUsageBytes', name: 'RAM', stroke: primaryColor }],
          yAxisDomain: [0, ramTotal] as [number, number],
          yAxisFormatter: (value: number) => formatBytesForDisplay(value),
          tooltipValueFormatter: (value: number) => {
            if (ramTotal === 0) return 'N/A';
            const percentage = ((value / ramTotal) * 100).toFixed(1);
            return [`${percentage}% (${formatBytesForDisplay(value)})`, 'RAM'];
          },
        };
      case 'network':
        return {
          title: t('components.realtimeMetricChart.networkChartTitle'),
          lines: [
            { dataKey: 'networkRxInstantBps', name: t('components.realtimeMetricChart.download'), stroke: primaryColor },
            { dataKey: 'networkTxInstantBps', name: t('components.realtimeMetricChart.upload'), stroke: secondaryColor },
          ],
          showLegend: true,
          yAxisFormatter: (value: number) => formatNetworkSpeed(value),
          tooltipValueFormatter: (value: number, name: string) => [formatNetworkSpeed(value), name],
        };
      case 'disk':
        return {
          title: t('components.realtimeMetricChart.diskIoChartTitle'),
          lines: [
            { dataKey: 'diskIoReadBps', name: t('components.realtimeMetricChart.read'), stroke: '#ff7300' },
            { dataKey: 'diskIoWriteBps', name: t('components.realtimeMetricChart.write'), stroke: '#387908' },
          ],
          showLegend: true,
          yAxisFormatter: (value: number) => formatNetworkSpeed(value),
          tooltipValueFormatter: (value: number, name: string) => [formatNetworkSpeed(value), name],
        };
    }
  }, [metricType, t, ramTotal]);

  const processedData = useMemo(() => {
    const now = Date.now();
    const cutoff = now - REALTIME_WINDOW_SECONDS * 1000;
    return data
      .map(p => ({ ...p, time: new Date(p.time).getTime() }))
      .filter(p => p.time >= cutoff);
  }, [data]);

  if (status === 'loading') {
    return (
      <div className={`flex flex-col items-center justify-center ${className}`}>
        {showTitle && <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{chartConfig.title}</h3>}
        <div className="relative flex-grow flex items-center justify-center w-full">
          <p>{t('components.realtimeMetricChart.loadingInitialData')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col ${className}`}>
      {showTitle && <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{chartConfig.title}</h3>}
      <div className="relative flex-grow">
        <ServerMetricsChart
          data={processedData}
          lines={chartConfig.lines}
          showLegend={chartConfig.showLegend}
          showYAxis={showYAxis}
          showXAxis={showXAxis}
          yAxisDomain={chartConfig.yAxisDomain}
          yAxisFormatter={chartConfig.yAxisFormatter}
          xAxisDomain={timeDomain}
          xAxisFormatter={(tick) => new Date(tick).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })}
          tooltipLabelFormatter={(label) => new Date(label).toLocaleTimeString()}
          tooltipValueFormatter={chartConfig.tooltipValueFormatter}
          noDataMessage={t('components.realtimeMetricChart.noRealtimeData')}
        />
      </div>
    </div>
  );
};

export default RealtimeMetricChart;