import type { TFunction } from 'i18next';
import { formatBytesForDisplay, formatNetworkSpeed } from '@/utils/vpsUtils';

// Define a more specific type for metric types we can handle
export type PerformanceMetricType = 'cpu' | 'ram' | 'network' | 'disk';
export type ServiceMetricType = 'service-latency';
export type MetricType = PerformanceMetricType | ServiceMetricType;


// Options for the configuration factory
interface ChartConfigOptions {
  metricType: MetricType;
  t: TFunction;
  // ramTotal is needed for tooltip calculations
  ramTotal?: number;
}

// The return type for a single chart configuration
export interface ChartConfig {
  title: string;
  lines: { dataKey: string; name: string; stroke: string }[];
  yAxisDomain?: [number | string, number | string];
  yAxisFormatter?: (value: number) => string;
  tooltipValueFormatter?: (value: number, name: string) => React.ReactNode | [React.ReactNode, React.ReactNode];
  chartType?: 'line' | 'area';
  showLegend?: boolean;
}

/**
 * Factory function to generate chart configurations.
 * Centralizes the logic for chart appearance and behavior.
 * @param options - The configuration options.
 * @returns A configuration object for the ServerMetricsChart component.
 */
export const getChartConfig = (options: ChartConfigOptions): ChartConfig => {
  const { metricType, t, ramTotal = 0 } = options;

  const primaryColor = '#8884d8';
  const secondaryColor = '#82ca9d';

  switch (metricType) {
    case 'cpu':
      return {
        title: t('vpsDetailPage.performanceMetrics.cpuUsageChartTitle'),
        lines: [{ dataKey: 'cpuUsagePercent', name: 'CPU', stroke: primaryColor }],
        yAxisDomain: [0, 100],
        yAxisFormatter: (value: number) => `${value}%`,
        tooltipValueFormatter: (value: number) => [`${value.toFixed(1)}%`, 'CPU'],
        chartType: 'area',
        showLegend: false,
      };

    case 'ram':
      return {
        title: t('vpsDetailPage.performanceMetrics.memoryUsageChartTitle'),
        lines: [{ dataKey: 'memoryUsagePercent', name: 'RAM', stroke: primaryColor }],
        yAxisDomain: [0, 100],
        yAxisFormatter: (value: number) => `${value}%`,
        tooltipValueFormatter: (value: number) => {
          if (ramTotal === 0) return [`${value.toFixed(1)}%`, 'RAM'];
          const absoluteValue = (value / 100) * ramTotal;
          return [`${value.toFixed(1)}% (${formatBytesForDisplay(absoluteValue)})`, 'RAM'];
        },
        chartType: 'area',
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
        chartType: 'line',
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
        chartType: 'line',
        showLegend: true,
      };
    
    case 'service-latency':
        return {
            title: t('vps.serviceMonitor'),
            lines: [], // Lines will be dynamically generated
            yAxisFormatter: (value: number) => `${value.toFixed(0)}ms`,
            tooltipValueFormatter: (value: number, name: string) => [`${value.toFixed(0)}ms`, name],
            chartType: 'line',
            showLegend: true,
        };

    default:
      // Fallback for unknown metric types
      return {
        title: 'Unknown Metric',
        lines: [],
      };
  }
};