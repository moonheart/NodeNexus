import React, { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getMonitorById, getMonitorResults } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorResult } from '../types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceArea } from 'recharts';
import websocketService from '../services/websocketService';
import { ArrowLeftIcon } from '../components/Icons';

const TIME_RANGE_OPTIONS = [
  { label: '实时', value: 'realtime' as const },
  { label: '1H', value: '1h' as const },
  { label: '6H', value: '6h' as const },
  { label: '24H', value: '24h' as const },
  { label: '7D', value: '7d' as const },
];
type TimeRangeOption = typeof TIME_RANGE_OPTIONS[number]['value'];


const ServiceMonitorDetailPage: React.FC = () => {
  const { monitorId } = useParams<{ monitorId: string }>();
  const [monitor, setMonitor] = useState<ServiceMonitor | null>(null);
  const [results, setResults] = useState<ServiceMonitorResult[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('realtime');

  interface ChartDataPoint {
    time: string;
    [agentName: string]: number | null | string;
  }
  const [chartData, setChartData] = useState<ChartDataPoint[]>([]);
  const [agentColors, setAgentColors] = useState<Record<string, string>>({});

  const AGENT_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff8042', '#0088FE', '#00C49F'];

  const processAndSetData = (data: ServiceMonitorResult[]) => {
    const sortedData = data.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
    setResults(sortedData);

    const groupedByAgent = sortedData.reduce((acc, result) => {
      const agentName = result.agentName;
      if (!acc[agentName]) {
        acc[agentName] = [];
      }
      acc[agentName].push(result);
      return acc;
    }, {} as Record<string, ServiceMonitorResult[]>);

    const newAgentColors = { ...agentColors };
    let colorIndex = Object.keys(newAgentColors).length;
    Object.keys(groupedByAgent).forEach(agentName => {
      if (!newAgentColors[agentName]) {
        newAgentColors[agentName] = AGENT_COLORS[colorIndex % AGENT_COLORS.length];
        colorIndex++;
      }
    });
    setAgentColors(newAgentColors);

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

  const timeRangeToMillis: Record<Exclude<TimeRangeOption, 'realtime'>, number> = { '1h': 36e5, '6h': 216e5, '24h': 864e5, '7d': 6048e5 };

  useEffect(() => {
    if (!monitorId) return;

    const fetchInitialData = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(parseInt(monitorId, 10));
        setMonitor(monitorData);

        let resultsData: ServiceMonitorResult[];
        if (selectedTimeRange === 'realtime') {
          resultsData = await getMonitorResults(parseInt(monitorId, 10), undefined, undefined, 300);
        } else {
          const endTime = new Date();
          const startTime = new Date(endTime.getTime() - timeRangeToMillis[selectedTimeRange]);
          resultsData = await getMonitorResults(parseInt(monitorId, 10), startTime.toISOString(), endTime.toISOString());
        }

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
  }, [monitorId, selectedTimeRange]);

  useEffect(() => {
    if (!monitorId) return;

    const handleNewResult = (result: ServiceMonitorResult) => {
      if (result.monitorId !== parseInt(monitorId, 10) || selectedTimeRange !== 'realtime') {
        return;
      }

      setResults(prevResults => {
        const updatedResults = [...prevResults, result].sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
        return updatedResults.length > 300 ? updatedResults.slice(updatedResults.length - 300) : updatedResults;
      });
    };

    websocketService.on('service_monitor_result', handleNewResult);

    return () => {
      websocketService.off('service_monitor_result', handleNewResult);
    };
  }, [monitorId, selectedTimeRange]);

  useEffect(() => {
    // Re-process chart data whenever results change
    if (results.length > 0) {
      processAndSetData(results);
    }
  }, [results]);


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
      <div className="flex justify-between items-center mb-4">
        <div>
            <h1 className="text-3xl font-bold mb-2">{monitor.name}</h1>
            <p className="text-lg text-gray-600">{monitor.monitorType.toUpperCase()} - {monitor.target}</p>
        </div>
        <Link to="/service-monitoring" className="inline-flex items-center bg-slate-200 hover:bg-slate-300 text-slate-700 font-medium py-1.5 px-3.5 rounded-lg transition-colors text-sm">
            <ArrowLeftIcon className="w-4 h-4 mr-1.5" /> Back to List
        </Link>
      </div>
      
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
        <div className="flex flex-col sm:flex-row justify-between items-center mb-4">
            <h2 className="text-2xl font-bold">Monitoring Results</h2>
            <div className="flex items-center space-x-1 mt-3 sm:mt-0 p-1 bg-slate-100 rounded-lg">
                {TIME_RANGE_OPTIONS.map(period => (
                <button
                    key={period.value}
                    onClick={() => setSelectedTimeRange(period.value)}
                    aria-pressed={selectedTimeRange === period.value}
                    className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${selectedTimeRange === period.value ? 'bg-indigo-600 text-white shadow' : 'text-slate-600 hover:bg-slate-200'}`}
                >
                    {period.label}
                </button>
                ))}
            </div>
        </div>
        <div className="bg-white p-4 rounded-lg shadow h-96">
          {results.length > 0 ? (
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" tick={{ fontSize: 11 }} />
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