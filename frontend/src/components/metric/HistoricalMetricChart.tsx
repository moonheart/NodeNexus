import React, { useEffect, useMemo, useState } from 'react';
import type { PerformanceMetricPoint } from '../../types';
import ServerMetricsChart from './ServerMetricsChart';
import { getVpsMetrics } from '@/services/metricsService';
import { getTimeRangeDetails, type TimeRangeValue } from '@/components/TimeRangeSelector';
import { useTranslation } from 'react-i18next';
import { formatBytesForDisplay, formatNetworkSpeed } from '@/utils/vpsUtils';
import { useServerListStore } from '@/store/serverListStore';

type ChartData = {
  time: number;
  [key: string]: number | null;
};

interface HistoricalPerformanceChartProps {
  vpsId: number;
  metricType: 'cpu' | 'ram' | 'network' | 'disk';
  timeRange: TimeRangeValue;
}



const HistoricalPerformanceChart: React.FC<HistoricalPerformanceChartProps> = ({
  vpsId,
  metricType,
  timeRange,
}) => {
  const { t } = useTranslation();
  const [metrics, setMetrics] = useState<PerformanceMetricPoint[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const latestMetric = useServerListStore(
    state => state.latestMetrics[vpsId]
  );
  const ramTotal = latestMetric?.memoryTotalBytes ?? 0;

  useEffect(() => {
    const fetchMetrics = async () => {
      setLoading(true);
      setError(null);
      try {
        const timeRangeDetails = getTimeRangeDetails(timeRange);
        const data = await getVpsMetrics(vpsId, timeRangeDetails.startTime, timeRangeDetails.endTime, timeRangeDetails.interval);
        setMetrics(data);
      } catch (err) {
        console.error(`Failed to fetch historical performance metrics for VPS ${vpsId} [${timeRange}]:`, err);
        setError(t('vps.errors.fetchMetricsFailed'));
        setMetrics([]);
      } finally {
        setLoading(false);
      }
    };

    fetchMetrics();
  }, [vpsId, timeRange, t]);

  const chartConfig = useMemo(() => {
    const primaryColor = '#8884d8';
    const secondaryColor = '#82ca9d';

    switch (metricType) {
      case 'cpu':
        return {
          title: t('vpsDetailPage.performanceMetrics.cpuUsageChartTitle'),
          lines: [{ dataKey: 'cpuUsagePercent', name: 'CPU', stroke: primaryColor }],
          yAxisDomain: [0, 100] as [number, number],
          yAxisFormatter: (value: number) => `${value}%`,
          tooltipValueFormatter: (value: number) => [`${value.toFixed(1)}%`, 'CPU'],
          chartType: 'area' as const,
          showLegend: false,
        };
      case 'ram':
        return {
          title: t('vpsDetailPage.performanceMetrics.memoryUsageChartTitle'),
          lines: [{ dataKey: 'memoryUsagePercent', name: 'RAM', stroke: primaryColor }],
          yAxisDomain: [0, 100] as [number, number],
          yAxisFormatter: (value: number) => `${value}%`,
          tooltipValueFormatter: (value: number) => {
            if (ramTotal === 0) return [`${value.toFixed(1)}%`, 'RAM'];
            const absoluteValue = (value / 100) * ramTotal;
            return [`${value.toFixed(1)}% (${formatBytesForDisplay(absoluteValue)})`, 'RAM'];
          },
          chartType: 'area' as const,
          showLegend: false,
        };
      case 'network':
        return {
          title: t('vpsDetailPage.networkChart.title'),
          lines: [
            { dataKey: 'networkRxInstantBps', name: t('vpsDetailPage.networkChart.download'), stroke: primaryColor },
            { dataKey: 'networkTxInstantBps', name: t('vpsDetailPage.networkChart.upload'), stroke: secondaryColor },
          ],
          yAxisFormatter: (value: number) => formatNetworkSpeed(value),
          tooltipValueFormatter: (value: number, name: string) => [formatNetworkSpeed(value), name],
          chartType: 'line' as const,
          showLegend: true,
        };
      case 'disk':
        return {
          title: t('vpsDetailPage.diskIoChart.title'),
          lines: [
            { dataKey: 'diskIoReadBps', name: t('vpsDetailPage.diskIoChart.read'), stroke: '#ff7300' },
            { dataKey: 'diskIoWriteBps', name: t('vpsDetailPage.diskIoChart.write'), stroke: '#387908' },
          ],
          yAxisFormatter: (value: number) => formatNetworkSpeed(value),
          tooltipValueFormatter: (value: number, name: string) => [formatNetworkSpeed(value), name],
          chartType: 'line' as const,
          showLegend: true,
        };
    }
  }, [metricType, t, ramTotal]);

  const chartData = useMemo(() => {
    const calculateMemoryUsagePercent = (p: PerformanceMetricPoint): number | null => {
        if (p.memoryUsageBytes != null && p.memoryTotalBytes != null && p.memoryTotalBytes > 0) {
            return (p.memoryUsageBytes / p.memoryTotalBytes) * 100;
        }
        return null;
    };

    const sortedMetrics = [...metrics].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());

    return sortedMetrics.map((point): ChartData => {
      const dataPoint: ChartData = { time: new Date(point.time).getTime() };
      
      chartConfig.lines.forEach(line => {
        const key = line.dataKey;
        if (key === 'memoryUsagePercent') {
            dataPoint[key] = calculateMemoryUsagePercent(point);
        } else {
            const value = point[key as keyof PerformanceMetricPoint];
            dataPoint[key] = (typeof value === 'number') ? value : null;
        }
      });
      
      return dataPoint;
    });
  }, [metrics, chartConfig.lines]);

  return (
    <div className="flex h-72 w-full flex-col">
      <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{chartConfig.title}</h3>
      <div className="relative flex-grow">
        <ServerMetricsChart
          data={chartData}
          loading={loading}
          error={error}
          chartType={chartConfig.chartType}
          lines={chartConfig.lines.map(l => ({...l, dataKey: l.dataKey as string}))}
          showLegend={chartConfig.showLegend}
          yAxisDomain={chartConfig.yAxisDomain}
          yAxisFormatter={chartConfig.yAxisFormatter}
          xAxisFormatter={(tick) => new Date(tick).toLocaleString()}
          tooltipLabelFormatter={(label) => new Date(label).toLocaleString()}
          tooltipValueFormatter={chartConfig.tooltipValueFormatter}
        />
      </div>
    </div>
  );
};

export default HistoricalPerformanceChart;