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