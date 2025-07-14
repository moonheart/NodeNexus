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

/**
 * Fetches time series results for a specific service monitor.
 * @param id - The ID of the service monitor.
 * @param startTime - The start of the time range (ISO string).
 * @param endTime - The end of the time range (ISO string).
 * @param interval - Optional. The aggregation interval (e.g., "30s", "5m", "1h"). If null, raw data is fetched.
 * @returns A promise that resolves to an array of service monitor results.
 */
export const getMonitorResults = async (
  id: number,
  startTime: string,
  endTime: string,
  interval: string | null
): Promise<ServiceMonitorResult[]> => {
  const params: {
    startTime: string;
    endTime: string;
    interval?: string;
  } = {
    startTime,
    endTime,
  };

  if (interval) {
    params.interval = interval;
  }

  const response = await apiClient.get(`/monitors/${id}/results`, { params });
  return response.data;
};

/**
 * Fetches time series results for all monitors associated with a specific VPS.
 * @param vpsId - The ID of the VPS.
 * @param startTime - The start of the time range (ISO string).
 * @param endTime - The end of the time range (ISO string).
 * @param interval - Optional. The aggregation interval (e.g., "30s", "5m", "1h"). If null, raw data is fetched.
 * @returns A promise that resolves to an array of service monitor results.
 */
export const getMonitorResultsByVpsId = async (
  vpsId: number | string,
  startTime: string,
  endTime: string,
  interval: string | null
): Promise<ServiceMonitorResult[]> => {
  const params: {
    startTime: string;
    endTime: string;
    interval?: string;
  } = {
    startTime,
    endTime,
  };

  if (interval) {
    params.interval = interval;
  }

  const response = await apiClient.get(`/vps/${vpsId}/monitor-results`, { params });
  return response.data;
};

export const getMonitorsByVpsId = async (vpsId: number | string): Promise<ServiceMonitor[]> => {
  const response = await apiClient.get(`/vps/${vpsId}/monitors`);
  return response.data;
};