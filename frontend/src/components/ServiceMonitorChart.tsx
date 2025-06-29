import React, { useMemo, useState } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea, type LegendProps } from 'recharts';
import type { ServiceMonitorResult } from '../types';
import type { ValueType } from 'recharts/types/component/DefaultTooltipContent';

// Helper to format latency for tooltips
const formatLatencyForTooltip = (value: ValueType) => {
  if (typeof value === 'number') {
    return `${value.toFixed(0)} ms`;
  }
  return `${value}`;
};

// Helper to format date for XAxis
const formatDateTick = (tickItem: string) => {
  const date = new Date(tickItem);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
};

// Helper to format the label in tooltips to local time
const formatTooltipLabel = (label: string) => {
  const date = new Date(label);
  return date.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
};

const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F', '#FFBB28', '#FF8042'];

export const ServiceMonitorChart: React.FC<{ results: ServiceMonitorResult[] }> = React.memo(({ results }) => {
    const [hiddenLines, setHiddenLines] = useState<Record<string, boolean>>({});

    const { chartData, monitorLines, downtimeAreas } = useMemo(() => {
        if (!results || results.length === 0) {
            return { chartData: [], monitorLines: [], downtimeAreas: [] };
        }

        const sortedResults = [...results].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());

        const groupedByMonitorId = sortedResults.reduce((acc, result) => {
            const monitorId = result.monitorId;
            if (!acc[monitorId]) {
                acc[monitorId] = [];
            }
            acc[monitorId].push(result);
            return acc;
        }, {} as Record<number, ServiceMonitorResult[]>);

        const monitorLines: { dataKey: string; name: string; stroke: string }[] = [];
        let colorIndex = 0;
        for (const monitorId in groupedByMonitorId) {
            if (Object.prototype.hasOwnProperty.call(groupedByMonitorId, monitorId)) {
                const firstResult = groupedByMonitorId[monitorId][0];
                const monitorName = firstResult.monitorName || `Monitor #${monitorId}`;
                const color = AGENT_COLORS[colorIndex % AGENT_COLORS.length];
                
                monitorLines.push({
                    dataKey: `monitor_${monitorId}`,
                    name: monitorName,
                    stroke: color,
                });
                colorIndex++;
            }
        }

        const timePoints = [...new Set(sortedResults.map(r => new Date(r.time).toISOString()))].sort();

        const chartData = timePoints.map(time => {
            const point: { time: string; [key: string]: number | null | string } = { time };
            for (const monitorId in groupedByMonitorId) {
                const dataKey = `monitor_${monitorId}`;
                const resultForTime = groupedByMonitorId[monitorId].find(r => new Date(r.time).toISOString() === time);
                point[dataKey] = resultForTime && resultForTime.isUp ? resultForTime.latencyMs : null;
            }
            return point;
        });

        const areas: { x1: string, x2: string }[] = [];
        let downtimeStart: string | null = null;

        for (let i = 0; i < timePoints.length; i++) {
            const time = timePoints[i];
            const isAnyDown = Object.values(groupedByMonitorId).some(monitorResults =>
                monitorResults.some(r => new Date(r.time).toISOString() === time && !r.isUp)
            );

            if (isAnyDown && !downtimeStart) {
                downtimeStart = time;
            } else if (!isAnyDown && downtimeStart) {
                const prevTime = i > 0 ? timePoints[i-1] : downtimeStart;
                areas.push({ x1: downtimeStart, x2: prevTime });
                downtimeStart = null;
            }
        }
        if (downtimeStart) {
            areas.push({ x1: downtimeStart, x2: timePoints[timePoints.length - 1] });
        }

        return { chartData, monitorLines, downtimeAreas: areas };
    }, [results]);

    const handleLegendClick: LegendProps['onClick'] = (data) => {
        const dataKey = data.dataKey as string;
        if (typeof dataKey === 'string') {
            setHiddenLines(prev => ({ ...prev, [dataKey]: !prev[dataKey] }));
        }
    };

    const renderLegendText: LegendProps['formatter'] = (value, entry) => {
        const { color, dataKey } = entry;
        const isHidden = typeof dataKey === 'string' && hiddenLines[dataKey];
        return <span style={{ color: isHidden ? '#A0A0A0' : color || '#000', cursor: 'pointer' }}>{value}</span>;
    };

    if (results.length === 0) {
        return <p className="text-center text-muted-foreground pt-16">此 VPS 无可用的服务监控数据。</p>;
    }

    return (
        <div className="h-80">
            <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData} margin={{ top: 5, right: 20, left: 5, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" tickFormatter={formatDateTick} tick={{ fontSize: 11 }} />
                    <YAxis tickFormatter={(tick) => `${tick} ms`} width={80} tick={{ fontSize: 11 }} />
                    <Tooltip formatter={formatLatencyForTooltip} labelFormatter={formatTooltipLabel} contentStyle={{ backgroundColor: 'hsl(var(--background) / 0.8)', backdropFilter: 'blur(2px)', borderRadius: 'var(--radius)', fontSize: '0.8rem' }} />
                    <Legend wrapperStyle={{ fontSize: '0.8rem' }} onClick={handleLegendClick} formatter={renderLegendText} />
                    {downtimeAreas.map((area, index) => (
                        <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="hsl(var(--destructive))" fillOpacity={0.15} ifOverflow="visible" />
                    ))}
                    {monitorLines.map((line) => (
                        <Line
                            key={line.dataKey}
                            type="monotone"
                            dataKey={line.dataKey}
                            name={line.name}
                            stroke={hiddenLines[line.dataKey] ? 'transparent' : line.stroke}
                            dot={false}
                            connectNulls={true}
                            strokeWidth={2}
                            isAnimationActive={false}
                        />
                    ))}
                </LineChart>
            </ResponsiveContainer>
        </div>
    );
});