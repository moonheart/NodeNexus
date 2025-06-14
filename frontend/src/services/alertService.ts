import apiClient from './apiClient';
import type { AlertRule, CreateAlertRulePayload, UpdateAlertRulePayload } from '../types';

// Assuming your API endpoint for alert rules is /api/alerts

/**
 * Fetches all alert rules for the current user.
 */
export const getAllAlertRules = async (): Promise<AlertRule[]> => {
  const response = await apiClient.get<AlertRule[]>('/alerts');
  return response.data;
};

/**
 * Fetches a single alert rule by its ID.
 * @param id - The ID of the alert rule.
 */
export const getAlertRuleById = async (id: number): Promise<AlertRule> => {
  const response = await apiClient.get<AlertRule>(`/alerts/${id}`);
  return response.data;
};

/**
 * Creates a new alert rule.
 * @param ruleData - The data for the new alert rule.
 */
export const createAlertRule = async (ruleData: CreateAlertRulePayload): Promise<AlertRule> => {
  const response = await apiClient.post<AlertRule>('/alerts', ruleData);
  return response.data;
};

/**
 * Updates an existing alert rule.
 * @param id - The ID of the alert rule to update.
 * @param ruleData - The data to update.
 */
export const updateAlertRule = async (id: number, ruleData: UpdateAlertRulePayload): Promise<AlertRule> => {
  const response = await apiClient.put<AlertRule>(`/alerts/${id}`, ruleData);
  return response.data;
};

/**
 * Deletes an alert rule.
 * @param id - The ID of the alert rule to delete.
 */
export const deleteAlertRule = async (id: number): Promise<void> => {
  await apiClient.delete(`/alerts/${id}`);
};

/**
 * Updates the status (isActive) of an existing alert rule.
 * @param id - The ID of the alert rule to update.
 * @param isActive - The new status for the alert rule.
 */
export const updateAlertRuleStatus = async (id: number, isActive: boolean): Promise<AlertRule> => {
  const response = await apiClient.put<AlertRule>(`/alerts/${id}/status`, { isActive });
  return response.data;
};