import React, { useMemo } from 'react';
import type { PerformanceMetricPoint } from '../types';
import ServerMetricsChart from './ServerMetricsChart';

type ChartLineConfig = {
  dataKey: keyof PerformanceMetricPoint;
  name: string;
  stroke: string;
};

// This defines the structure of the data points that the chart component expects.
// It must have a 'time' property, and can have any other string keys with number values.
type ChartData = {
  time: number;
  [key: string]: number | null | undefined;
};

interface HistoricalPerformanceChartProps {
  title: string;
  metrics: PerformanceMetricPoint[];
  loading: boolean;
  error: string | null;
  lines: ChartLineConfig[];
  yAxisDomain?: [number | string, number | string];
  yAxisFormatter?: (tick: number) => string;
  tooltipValueFormatter?: (value: number, name: string) => React.ReactNode | [React.ReactNode, React.ReactNode];
  chartType?: 'line' | 'area';
  showLegend?: boolean;
}

const HistoricalPerformanceChart: React.FC<HistoricalPerformanceChartProps> = ({
  title,
  metrics,
  loading,
  error,
  lines,
  yAxisDomain,
  yAxisFormatter,
  tooltipValueFormatter,
  chartType = 'area',
  showLegend = false,
}) => {

  const chartData = useMemo(() => {
    const calculateMemoryUsagePercent = (p: PerformanceMetricPoint): number | null => {
        // Historical data from the API also uses 'memoryUsageBytes' for the aggregated value.
        if (p.memoryUsageBytes != null && p.memoryTotalBytes != null && p.memoryTotalBytes > 0) {
            return (p.memoryUsageBytes / p.memoryTotalBytes) * 100;
        }
        return null;
    };

    // Create a sorted copy to avoid mutating the original prop array
    const sortedMetrics = [...metrics].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());

    return sortedMetrics.map((point): ChartData => {
      const dataPoint: ChartData = { time: new Date(point.time).getTime() };
      
      lines.forEach(line => {
        // Special handling for calculated fields like memory percentage
        if (line.dataKey === 'memoryUsagePercent') {
            dataPoint[line.dataKey] = calculateMemoryUsagePercent(point);
        } else {
            const value = point[line.dataKey];
            if (typeof value === 'number' || value === null || typeof value === 'undefined') {
              dataPoint[line.dataKey as string] = value;
            }
        }
      });
      
      return dataPoint;
    });
  }, [metrics, lines]);

  return (
    <div className="flex h-72 w-full flex-col">
      <h3 className="text-lg font-semibold text-center mb-2 flex-shrink-0">{title}</h3>
      <div className="relative flex-grow">
        <ServerMetricsChart
          data={chartData}
          loading={loading}
          error={error}
          chartType={chartType}
          lines={lines.map(l => ({...l, dataKey: l.dataKey as string}))}
          showLegend={showLegend}
          yAxisDomain={yAxisDomain}
          yAxisFormatter={yAxisFormatter}
          xAxisFormatter={(tick) => new Date(tick).toLocaleString()}
          tooltipLabelFormatter={(label) => new Date(label).toLocaleString()}
          tooltipValueFormatter={tooltipValueFormatter}
        />
      </div>
    </div>
  );
};

export default HistoricalPerformanceChart;