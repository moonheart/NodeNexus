import apiClient from './apiClient';
import type { ServiceMonitor, ServiceMonitorResult, ServiceMonitorInput } from '../types';

export const getMonitors = async (): Promise<ServiceMonitor[]> => {
  const response = await apiClient.get('/monitors');
  return response.data;
};

export const getMonitorById = async (id: number): Promise<ServiceMonitor> => {
    const response = await apiClient.get(`/monitors/${id}`);
    return response.data;
};

export const createMonitor = async (data: ServiceMonitorInput): Promise<ServiceMonitor> => {
  const response = await apiClient.post('/monitors', data);
  return response.data;
};

export const updateMonitor = async (id: number, data: ServiceMonitorInput): Promise<ServiceMonitor> => {
  const response = await apiClient.put(`/monitors/${id}`, data);
  return response.data;
};

export const deleteMonitor = async (id: number): Promise<void> => {
  await apiClient.delete(`/monitors/${id}`);
};

export const getMonitorResults = async (
  id: number,
  startTime?: string,
  endTime?: string,
  points?: number
): Promise<ServiceMonitorResult[]> => {
  const params = new URLSearchParams();
  if (startTime) params.append('start_time', startTime);
  if (endTime) params.append('end_time', endTime);
  if (points) params.append('points', points.toString());

  const response = await apiClient.get(`/monitors/${id}/results`, { params });
  return response.data;
};
export const getMonitorResultsByVpsId = async (
  vpsId: number | string,
  startTime?: string,
  endTime?: string,
  points?: number
): Promise<ServiceMonitorResult[]> => {
  const params = new URLSearchParams();
  if (startTime) params.append('start_time', startTime);
  if (endTime) params.append('end_time', endTime);
  if (points) params.append('points', points.toString());

  const response = await apiClient.get(`/vps/${vpsId}/monitor-results`, { params });
  return response.data;
};