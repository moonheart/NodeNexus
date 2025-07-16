import React from 'react';
import {
  AreaChart,
  Area,
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  Legend,
} from 'recharts';
import { useTranslation } from 'react-i18next';
import { Skeleton } from "@/components/ui/skeleton";

type ChartLine = {
  dataKey: string;
  name: string;
  stroke: string;
  dot?: boolean;
  isAnimationActive?: boolean;
};

type ChartDataPoint = {
  time: number;
  [key: string]: number | null | undefined;
};

interface ServerMetricsChartProps {
  data: ChartDataPoint[];
  lines: ChartLine[];
  chartType?: 'line' | 'area';
  yAxisFormatter?: (value: number) => string;
  yAxisDomain?: [number | string, number | string];
  xAxisFormatter?: (value: number) => string;
  xAxisDomain?: [number | string, number | string];
  tooltipLabelFormatter?: (label: number) => React.ReactNode;
  tooltipValueFormatter?: (value: number, name: string) => React.ReactNode | [React.ReactNode, React.ReactNode];
  showLegend?: boolean;
  showYAxis?: boolean;
  showXAxis?: boolean;
  loading?: boolean;
  error?: string | null;
  noDataMessage?: string;
}

const ServerMetricsChart: React.FC<ServerMetricsChartProps> = ({
  data,
  lines,
  chartType = 'area',
  yAxisFormatter,
  yAxisDomain,
  xAxisFormatter,
  xAxisDomain,
  tooltipLabelFormatter,
  tooltipValueFormatter,
  showLegend = false,
  showYAxis = true,
  showXAxis = true,
  loading = false,
  error = null,
  noDataMessage,
}) => {
  const { t } = useTranslation();

  if (loading) {
    return <Skeleton className="h-full w-full" />;
  }

  if (error) {
    return <div className="h-full w-full flex items-center justify-center text-destructive text-sm">{error}</div>;
  }

  if (data.length === 0) {
    return (
      <div className="h-full w-full flex items-center justify-center text-muted-foreground text-sm">
        {noDataMessage || t('vps.noData')}
      </div>
    );
  }

  const ChartComponent = chartType === 'area' ? AreaChart : LineChart;
  const LineComponent = chartType === 'area' ? Area : Line;

  return (
    <ResponsiveContainer width="100%" height="100%">
      <ChartComponent data={data} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
        <CartesianGrid strokeDasharray="3 3" />
        
          <XAxis
            dataKey="time"
            type="number"
            domain={xAxisDomain || ['dataMin', 'dataMax']}
            tickFormatter={xAxisFormatter}
            tickLine={false}
            axisLine={false}
            hide={!showXAxis} 
            tickMargin={8}
            tick={{ fontSize: 12 }}
          />
        {showYAxis && (
          <YAxis
            domain={yAxisDomain}
            tickFormatter={yAxisFormatter}
            width={yAxisFormatter ? 60 : 40}
            tickLine={false}
            axisLine={false}
            tickMargin={8}
            tick={{ fontSize: 12 }}
          />
        )}
        <Tooltip
          contentStyle={{
            backgroundColor: 'hsl(var(--background) / 0.8)',
            backdropFilter: 'blur(8px)',
            borderRadius: 'var(--radius)',
            fontSize: '0.8rem',
            padding: '8px 12px',
            border: '1px solid hsl(var(--border))',
          }}
          labelFormatter={tooltipLabelFormatter}
          formatter={tooltipValueFormatter}
        />
        {showLegend && <Legend />}
        {lines.map((line) => (
          <LineComponent
            key={line.dataKey}
            type="monotone"
            dataKey={line.dataKey}
            name={line.name}
            stroke={line.stroke}
            fill={line.stroke}
            fillOpacity={chartType === 'area' ? 0.3 : 1}
            strokeWidth={1}
            dot={line.dot ?? false}
            isAnimationActive={false}
            connectNulls={true}
          />
        ))}
      </ChartComponent>
    </ResponsiveContainer>
  );
};

export default ServerMetricsChart;