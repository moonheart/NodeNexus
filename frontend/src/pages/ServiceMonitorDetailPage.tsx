import React, { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { getMonitorById, getMonitorResults } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorResult } from '../types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea } from 'recharts';

const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  interface ChartDataPoint {
    time: string;
    [agentName: string]: number | null | string;
  }
  const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
  const [agentColors, setAgentColors] = useState<Record<string, string>>({});

  const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

  useEffect(() => {
    if (!monitorId) return;

    const processAndSetData = (data: ServiceMonitorResult[]) => {
      const sortedData = data.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
      setResults(sortedData);

      // Group results by agent
      const groupedByAgent = sortedData.reduce((acc, result) => {
        const agentName = result.agentName;
        if (!acc[agentName]) {
          acc[agentName] = [];
        }
        acc[agentName].push(result);
        return acc;
      }, {} as Record<string, ServiceMonitorResult[]>);

      // Assign colors to agents
      const newAgentColors = { ...agentColors };
      let colorIndex = Object.keys(newAgentColors).length;
      Object.keys(groupedByAgent).forEach(agentName => {
        if (!newAgentColors[agentName]) {
          newAgentColors[agentName] = AGENT_COLORS[colorIndex % AGENT_COLORS.length];
          colorIndex++;
        }
      });
      setAgentColors(newAgentColors);

      // Transform data for the chart
      const formattedTimePoints = [...new Set(sortedData.map(r => new Date(r.time).toLocaleString()))];
      const finalChartData = formattedTimePoints.map(time => {
        const point: ChartDataPoint = { time };
        Object.keys(groupedByAgent).forEach(agentName => {
          const resultForTime = groupedByAgent[agentName].find(r => new Date(r.time).toLocaleString() === time);
          point[agentName] = resultForTime ? resultForTime.latencyMs : null;
        });
        return point;
      });
      setChartData(finalChartData);
    };

    const fetchInitialData = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(parseInt(monitorId, 10));
        const resultsData = await getMonitorResults(parseInt(monitorId, 10));
        setMonitor(monitorData);
        processAndSetData(resultsData);
        setError(null);
      } catch (err) {
        setError('Failed to fetch monitor details.');
        console.error(err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchInitialData();

    const intervalId = setInterval(async () => {
      try {
        const resultsData = await getMonitorResults(parseInt(monitorId, 10));
        processAndSetData(resultsData);
      } catch (err) {
        console.error("Failed to fetch updated results:", err);
      }
    }, 5000);

    return () => clearInterval(intervalId);
  }, [monitorId]);

  const getDowntimeAreas = () => {
    const areas = [];
    let inDowntime = false;
    let start: string | null = null;
    const sortedResults = [...results].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
    const formattedResults = sortedResults.map(r => ({ ...r, time: new Date(r.time).toLocaleString() }));

    for (let i = 0; i < formattedResults.length; i++) {
      const result = formattedResults[i];
      if (!result.isUp && !inDowntime) {
        inDowntime = true;
        start = result.time;
      } else if (result.isUp && inDowntime) {
        inDowntime = false;
        if (start) {
          areas.push({ x1: start, x2: result.time, y1: 0, y2: 'auto' });
          start = null;
        }
      }
    }

    if (inDowntime && start && formattedResults.length > 0) {
      areas.push({ x1: start, x2: formattedResults[formattedResults.length - 1].time, y1: 0, y2: 'auto' });
    }
    return areas;
  };

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div className="text-red-500">{error}</div>;
  if (!monitor) return <div>Monitor not found.</div>;

  const downtimeAreas = getDowntimeAreas();

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-3xl font-bold mb-2">{monitor.name}</h1>
      <p className="text-lg text-gray-600 mb-4">{monitor.monitorType.toUpperCase()} - {monitor.target}</p>
      
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8 text-center">
        <div className="bg-white p-4 rounded-lg shadow">
            <h3 className="text-sm font-medium text-gray-500">Frequency</h3>
            <p className="text-2xl font-semibold">{monitor.frequencySeconds}s</p>
        </div>
        <div className="bg-white p-4 rounded-lg shadow">
            <h3 className="text-sm font-medium text-gray-500">Timeout</h3>
            <p className="text-2xl font-semibold">{monitor.timeoutSeconds}s</p>
        </div>
        <div className="bg-white p-4 rounded-lg shadow">
            <h3 className="text-sm font-medium text-gray-500">Status</h3>
            <p className={`text-2xl font-semibold ${monitor.isActive ? 'text-green-500' : 'text-red-500'}`}>
                {monitor.isActive ? 'Active' : 'Inactive'}
            </p>
        </div>
      </div>

      <div className="mt-8">
        <h2 className="text-2xl font-bold mb-4">Monitoring Results</h2>
        <div className="bg-white p-4 rounded-lg shadow h-96">
          {results.length > 0 ? (
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" />
                <YAxis label={{ value: 'Latency (ms)', angle: -90, position: 'insideLeft' }} />
                <Tooltip />
                <Legend />
                {downtimeAreas.map((area, index) => (
                  <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="red" fillOpacity={0.1} />
                ))}
                {Object.keys(agentColors).map(agentName => (
                  <Line
                    key={agentName}
                    type="monotone"
                    dataKey={agentName}
                    name={agentName}
                    stroke={agentColors[agentName]}
                    dot={false}
                    activeDot={{ r: 8 }}
                    connectNulls={false}
                  />
                ))}
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <p>No monitoring results available yet.</p>
          )}
        </div>
      </div>
    </div>
  );
};

export default ServiceMonitorDetailPage;