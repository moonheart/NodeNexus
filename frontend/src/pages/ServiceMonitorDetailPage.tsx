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

  useEffect(() => {
    if (!monitorId) return;

    const fetchInitialData = async () => {
      try {
        setIsLoading(true);
        const monitorData = await getMonitorById(parseInt(monitorId, 10));
        const resultsData = await getMonitorResults(parseInt(monitorId, 10));
        setMonitor(monitorData);
        setResults(resultsData.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime()));
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
        setResults(resultsData.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime()));
      } catch (err) {
        console.error("Failed to fetch updated results:", err);
        // Optionally, set an error state for the update failure
      }
    }, 5000); // Refresh every 5 seconds

    return () => clearInterval(intervalId); // Cleanup on component unmount
  }, [monitorId]);

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div className="text-red-500">{error}</div>;
  }

  if (!monitor) {
    return <div>Monitor not found.</div>;
  }

  const getDowntimeAreas = () => {
    const areas = [];
    let inDowntime = false;
    let start: string | null = null;

    const formattedResults = [...results].map(r => ({ ...r, time: new Date(r.time).toLocaleString() }));

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

    // If the last period was downtime, close it off.
    if (inDowntime && start && formattedResults.length > 0) {
      areas.push({ x1: start, x2: formattedResults[formattedResults.length - 1].time, y1: 0, y2: 'auto' });
    }

    return areas;
  };

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
              <LineChart data={results.map(r => ({ ...r, time: new Date(r.time).toLocaleString() }))}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" />
                <YAxis label={{ value: 'Latency (ms)', angle: -90, position: 'insideLeft' }} />
                <Tooltip />
                <Legend />
                {downtimeAreas.map((area, index) => (
                  <ReferenceArea key={index} x1={area.x1} x2={area.x2} stroke="transparent" fill="red" fillOpacity={0.1} />
                ))}
                <Line type="monotone" dataKey="latencyMs" name="Latency (ms)" stroke="#8884d8" dot={false} activeDot={{ r: 8 }} />
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