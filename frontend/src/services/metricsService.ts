import apiClient from './apiClient';
import type { PerformanceMetricPoint } from '../types'; // Assuming type definition in index.ts

/**
 * Fetches time series performance metrics for a specific VPS.
 * @param vpsId - The ID of the VPS.
 * @param startTime - The start of the time range (ISO string).
 * @param endTime - The end of the time range (ISO string).
 * @param interval - The aggregation interval (e.g., "1m", "5m", "1h").
 * @returns A promise that resolves to an array of performance metric points.
 */
export const getVpsMetricsTimeseries = async (
  vpsId: number | string, // Allow string for vpsId from URL params
  startTime: string,
  endTime: string,
  interval: string
): Promise<PerformanceMetricPoint[]> => {
  try {
    const response = await apiClient.get<PerformanceMetricPoint[]>(
      `/api/vps/${vpsId}/metrics/timeseries`,
      {
        params: {
          start_time: startTime,
          end_time: endTime,
          interval: interval,
        },
      }
    );
    return response.data;
  } catch (error) {
    console.error('Error fetching VPS timeseries metrics:', error);
    // Consider how to handle errors, e.g., re-throw or return a specific error structure
    throw error;
  }
};

/**
 * Fetches the latest performance metrics for a specific VPS.
 * @param vpsId - The ID of the VPS.
 * @returns A promise that resolves to the latest performance metric object or null if not found.
 */
export const getLatestVpsMetrics = async (
  vpsId: number | string
): Promise<import('../types').LatestPerformanceMetric | null> => {
  try {
    const response = await apiClient.get<import('../types').LatestPerformanceMetric>(
      `/api/vps/${vpsId}/metrics/latest`
    );
    return response.data; // The backend returns the metric object directly, or null/404 if not found
  } catch (error: unknown) {
    // Type guard for AxiosError
    if (typeof error === 'object' && error !== null && 'response' in error) {
      const axiosError = error as { response?: { status: number } };
      if (axiosError.response && axiosError.response.status === 404) {
        console.warn(`No latest metrics found for VPS ${vpsId}`);
        return null;
      }
    }
    console.error(`Error fetching latest metrics for VPS ${vpsId}:`, error);
    throw error;
  }
};
/**
 * Fetches the latest N performance metrics for a specific VPS.
 * @param vpsId - The ID of the VPS.
 * @param count - The number of latest data points to fetch.
 * @returns A promise that resolves to an array of performance metric points.
 */
export const getLatestNMetrics = async (
  vpsId: number | string,
  count: number
): Promise<PerformanceMetricPoint[]> => {
  try {
    const response = await apiClient.get<PerformanceMetricPoint[]>(
      `/api/vps/${vpsId}/metrics/latest-n`,
      {
        params: {
          count: count,
        },
      }
    );
    return response.data;
  } catch (error) {
    console.error(`Error fetching latest ${count} metrics for VPS ${vpsId}:`, error);
    throw error;
  }
};