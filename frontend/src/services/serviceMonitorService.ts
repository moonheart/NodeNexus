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

export const getMonitorResults = async (id: number): Promise<ServiceMonitorResult[]> => {
    const response = await apiClient.get(`/monitors/${id}/results`);
    return response.data;
};