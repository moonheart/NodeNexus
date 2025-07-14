import apiClient from './apiClient';
import type { PerformanceMetricPoint } from '../types';

/**
 * Fetches time series performance metrics for a specific VPS.
 * This is now the single function for fetching performance data, supporting
 * both raw and aggregated data based on the presence of the interval.
 *
 * @param vpsId - The ID of the VPS.
 * @param startTime - The start of the time range (ISO string).
 * @param endTime - The end of the time range (ISO string).
 * @param interval - Optional. The aggregation interval (e.g., "30s", "5m", "1h"). If null or undefined, raw data is fetched.
 * @returns A promise that resolves to an array of performance metric points.
 */
export const getVpsMetrics = async (
  vpsId: number | string,
  startTime: string,
  endTime: string,
  interval: string | null
): Promise<PerformanceMetricPoint[]> => {
  try {
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

    // The backend now returns fields that directly map to PerformanceMetricPoint (all camelCase)
    const response = await apiClient.get<PerformanceMetricPoint[]>(
      `/vps/${vpsId}/metrics/timeseries`,
      { params }
    );
    
    // The backend DTO is now identical to the frontend type, so no mapping is needed.
    return response.data;
  } catch (error) {
    console.error('Error fetching VPS timeseries metrics:', error);
    // Consider how to handle errors, e.g., re-throw or return a specific error structure
    throw error;
  }
};