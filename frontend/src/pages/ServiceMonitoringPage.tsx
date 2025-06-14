import React, { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import ServiceMonitorModal from '../components/ServiceMonitorModal'; // Import the modal
import { createMonitor, updateMonitor, getMonitors } from '../services/serviceMonitorService';
import type { ServiceMonitor, ServiceMonitorInput } from '../types';
import toast from 'react-hot-toast';
import { deleteMonitor } from '../services/serviceMonitorService';


const ServiceMonitoringPage: React.FC = () => {
  const [monitors, setMonitors] = useState<ServiceMonitor[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingMonitor, setEditingMonitor] = useState<ServiceMonitor | null>(null);

  const fetchMonitors = async () => {
    try {
      setIsLoading(true);
      const data = await getMonitors();
      setMonitors(data);
      setError(null);
    } catch (err) {
      setError('Failed to fetch service monitors.');
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchMonitors();
  }, []);

  const handleOpenCreateModal = () => {
    setEditingMonitor(null);
    setIsModalOpen(true);
  };

  const handleOpenEditModal = (monitor: ServiceMonitor) => {
    setEditingMonitor(monitor);
    setIsModalOpen(true);
  };

  const handleDelete = async (id: number) => {
    if (window.confirm('Are you sure you want to delete this monitor?')) {
      try {
        await deleteMonitor(id);
        toast.success('Monitor deleted successfully!');
        fetchMonitors(); // Refresh the list
      } catch (err) {
        toast.error('Failed to delete monitor.');
        console.error(err);
      }
    }
  };

  const handleSave = async (data: ServiceMonitorInput, id?: number) => {
    try {
      if (id) {
        await updateMonitor(id, data);
        toast.success('Monitor updated successfully!');
      } else {
        await createMonitor(data);
        toast.success('Monitor created successfully!');
      }
      setIsModalOpen(false);
      fetchMonitors(); // Refresh the list
    } catch (err) {
      toast.error('Failed to save monitor.');
      console.error(err);
    }
  };

  return (
    <div className="container mx-auto p-4">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-2xl font-bold">Service Monitoring</h1>
        <button
          onClick={handleOpenCreateModal}
          className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700"
        >
          Create Monitor
        </button>
      </div>

      <ServiceMonitorModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSave={handleSave}
        monitorToEdit={editingMonitor}
      />

      {isLoading && <p>Loading monitors...</p>}
      {error && <p className="text-red-500">{error}</p>}
      
      {!isLoading && !error && (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {monitors.length > 0 ? (
        monitors.map((monitor) => (
          <div key={monitor.id} className="bg-white rounded-lg shadow flex flex-col transition-shadow hover:shadow-md">
            <Link to={`/monitors/${monitor.id}`} className="flex-grow p-4">
                <h2 className="text-lg font-semibold text-indigo-600 hover:underline">{monitor.name}</h2>
                <p className="text-sm text-gray-600 truncate" title={monitor.target}>{monitor.monitorType.toUpperCase()} - {monitor.target}</p>
                <p className="text-sm text-gray-500">Frequency: {monitor.frequencySeconds}s</p>
            </Link>
            <div className="px-4 pb-3 flex justify-end space-x-2">
                <button onClick={(e) => { e.stopPropagation(); handleOpenEditModal(monitor); }} className="text-sm bg-gray-200 text-gray-800 px-3 py-1 rounded-md hover:bg-gray-300">Edit</button>
                <button onClick={(e) => { e.stopPropagation(); handleDelete(monitor.id); }} className="text-sm bg-red-500 text-white px-3 py-1 rounded-md hover:bg-red-600">Delete</button>
            </div>
          </div>
        ))
          ) : (
            <p>No service monitors found.</p>
          )}
        </div>
      )}
    </div>
  );
};

export default ServiceMonitoringPage;