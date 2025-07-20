import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import websocketService from '../services/websocketService';
import { useAuthStore } from './authStore';
import type { VpsListItemResponse, FullServerListPushType, ViewMode, Tag, ServiceMonitorResult, PerformanceMetricPoint, PerformanceMetricBatch } from '../types';
import * as tagService from '../services/tagService';
import equal from 'fast-deep-equal';
import { getMonitorResults, getMonitorResultsByVpsId } from '../services/serviceMonitorService';
import { getVpsMetrics } from '../services/metricsService';

// --- Pub/Sub Systems ---
export type UnsubscribeFunction = () => void;

// --- Service Monitor Pub/Sub (Per VPS) ---
type VpsMonitorResultCallback = (results: ServiceMonitorResult[]) => void;
const vpsMonitorResultsCache: Record<number, ServiceMonitorResult[]> = {};
const vpsMonitorResultListeners: Record<number, Set<VpsMonitorResultCallback>> = {};
const vpsInitialDataPromises: Record<number, Promise<ServiceMonitorResult[]>> = {};

// --- Legacy Pub/Sub for ServiceMonitorDetailPage ---
type MonitorResultCallback = (results: ServiceMonitorResult[]) => void;
const monitorResultsCache: Record<number, ServiceMonitorResult[]> = {};
const monitorResultListeners: Record<number, Set<MonitorResultCallback>> = {};
const isFetchingInitialData: Record<number, boolean> = {};


export type ConnectionStatus =
    | 'disconnected'
    | 'connecting'
    | 'connected'
    | 'error'
    | 'reconnecting'
    | 'permanently_failed';

type DataStatus = 'idle' | 'loading' | 'success' | 'error';

interface VpsMetricsState {
  data: PerformanceMetricPoint[];
  status: DataStatus;
  error?: string | null;
}

export interface ServerListState {
    servers: VpsListItemResponse[];
    latestMetrics: Record<number, PerformanceMetricPoint>;
    initialVpsMetrics: Record<number, VpsMetricsState>; // New centralized state
    connectionStatus: ConnectionStatus;
    isLoading: boolean;
    error: string | null;
    viewMode: ViewMode;
    allTags: Tag[];
    isInitialized: boolean;
    fetchAllTags: () => Promise<void>;
    setViewMode: (mode: ViewMode) => void;
    init: () => void;
    disconnectWebSocket: () => void;

    // Service Monitor Pub/Sub Actions (per VPS for homepage)
    subscribeToVpsMonitorResults: (vpsId: number, callback: VpsMonitorResultCallback) => UnsubscribeFunction;
    getInitialVpsMonitorResults: (vpsId: number) => Promise<ServiceMonitorResult[]>;
    clearVpsMonitorResults: () => void;

    // New Performance Metrics Actions
    ensureInitialVpsPerformanceMetrics: (vpsId: number) => Promise<void>;
    clearVpsPerformanceMetrics: () => void;

    // Service Monitor Pub/Sub Actions (per Monitor for detail page)
    subscribeToMonitorResults: (monitorId: number, callback: MonitorResultCallback) => UnsubscribeFunction;
    getInitialMonitorResults: (monitorId: number, startTime: string, endTime: string, interval: string | null) => Promise<ServiceMonitorResult[]>;
    clearMonitorResults: () => void;

    // Internal actions
    _initializeWebSocket: (isAuthenticated: boolean) => void;
    _handleWebSocketOpen: () => void;
    _handleWebSocketMessage: (data: FullServerListPushType) => void;
    _handleServiceMonitorResult: (data: ServiceMonitorResult) => void;
    _handlePerformanceMetricBatch: (data: PerformanceMetricBatch) => void;
    _handleWebSocketClose: (data: { isIntentional: boolean; event: CloseEvent }) => void;
    _handleWebSocketError: (event: Event) => void;
    _handleWebSocketPermanentFailure: () => void;
}

export const useServerListStore = create<ServerListState>()(
  persist(
    (set, get) => ({
      servers: [],
      latestMetrics: {},
      initialVpsMetrics: {},
      connectionStatus: 'disconnected',
      isLoading: true,
      error: null,
      viewMode: 'card',
      allTags: [],
      isInitialized: false,

    // --- Service Monitor Pub/Sub Implementation (Per VPS) ---
    subscribeToVpsMonitorResults: (vpsId, callback) => {
        if (!vpsMonitorResultListeners[vpsId]) {
            vpsMonitorResultListeners[vpsId] = new Set();
        }
        vpsMonitorResultListeners[vpsId].add(callback);

        return () => {
            vpsMonitorResultListeners[vpsId]?.delete(callback);
            if (vpsMonitorResultListeners[vpsId]?.size === 0) {
                delete vpsMonitorResultListeners[vpsId];
            }
        };
    },

    getInitialVpsMonitorResults: async (vpsId) => {
        if (vpsMonitorResultsCache[vpsId]) {
            return vpsMonitorResultsCache[vpsId];
        }

        if (vpsId in vpsInitialDataPromises) {
            return vpsInitialDataPromises[vpsId];
        }

        const promise = (async () => {
            try {
                const endTime = new Date();
                const startTime = new Date(endTime.getTime() - 10 * 60 * 1000);
                const results = await getMonitorResultsByVpsId(vpsId, startTime.toISOString(), endTime.toISOString(), null);
                const sortedResults = results.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
                vpsMonitorResultsCache[vpsId] = sortedResults;
                return sortedResults;
            } catch (error) {
                console.error(`Failed to get initial monitor results for VPS ${vpsId}:`, error);
                return [];
            } finally {
                delete vpsInitialDataPromises[vpsId];
            }
        })();

        vpsInitialDataPromises[vpsId] = promise;
        return promise;
    },
    
    clearVpsMonitorResults: () => {
        Object.keys(vpsMonitorResultsCache).forEach(key => delete vpsMonitorResultsCache[Number(key)]);
        Object.keys(vpsMonitorResultListeners).forEach(key => delete vpsMonitorResultListeners[Number(key)]);
        console.log('Cleared all VPS service monitor caches and listeners.');
    },

    // --- New Performance Metrics Implementation ---
    ensureInitialVpsPerformanceMetrics: async (vpsId) => {
        const currentState = get().initialVpsMetrics[vpsId];

        if (currentState?.status === 'loading' || currentState?.status === 'success') {
            return; // Already loading or loaded
        }

        set(state => ({
            initialVpsMetrics: {
                ...state.initialVpsMetrics,
                [vpsId]: { data: [], status: 'loading' },
            },
        }));

        try {
            const endTime = new Date();
            const startTime = new Date(endTime.getTime() - 10 * 60 * 1000); // 10 minutes ago
            
            const metrics = await getVpsMetrics(vpsId, startTime.toISOString(), endTime.toISOString(), null);
            const sortedMetrics = metrics.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
            
            set(state => ({
                initialVpsMetrics: {
                    ...state.initialVpsMetrics,
                    [vpsId]: { data: sortedMetrics, status: 'success' },
                },
            }));
        } catch (error) {
            console.error(`Failed to get initial performance metrics for VPS ${vpsId}:`, error);
            set(state => ({
                initialVpsMetrics: {
                    ...state.initialVpsMetrics,
                    [vpsId]: { data: [], status: 'error', error: 'Failed to load data' },
                },
            }));
        }
    },

    clearVpsPerformanceMetrics: () => {
        set({ initialVpsMetrics: {} });
        console.log('Cleared all VPS performance metrics state.');
    },
    
    // --- Pub/Sub Implementation for ServiceMonitorDetailPage ---
    subscribeToMonitorResults: (monitorId, callback) => {
        if (!monitorResultListeners[monitorId]) {
            monitorResultListeners[monitorId] = new Set();
        }
        monitorResultListeners[monitorId].add(callback);

        return () => {
            monitorResultListeners[monitorId]?.delete(callback);
            if (monitorResultListeners[monitorId]?.size === 0) {
                delete monitorResultListeners[monitorId];
            }
        };
    },

    getInitialMonitorResults: async (monitorId, startTime, endTime, interval) => {
        if (isFetchingInitialData[monitorId]) {
            await new Promise(resolve => setTimeout(resolve, 100));
            return get().getInitialMonitorResults(monitorId, startTime, endTime, interval);
        }
        try {
            isFetchingInitialData[monitorId] = true;
            const results = await getMonitorResults(monitorId, startTime, endTime, interval);
            const sortedResults = results.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
            monitorResultsCache[monitorId] = sortedResults;
            return sortedResults;
        } catch (error) {
            console.error(`Failed to get initial monitor results for monitor ${monitorId}:`, error);
            return [];
        } finally {
            isFetchingInitialData[monitorId] = false;
        }
    },

    clearMonitorResults: () => {
        Object.keys(monitorResultsCache).forEach(key => delete monitorResultsCache[Number(key)]);
        Object.keys(monitorResultListeners).forEach(key => delete monitorResultListeners[Number(key)]);
        console.log('Cleared all service monitor caches and listeners.');
    },


    fetchAllTags: async () => {
        try {
            const tags = await tagService.getTags();
            set({ allTags: tags });
        } catch (error) {
            console.error('Failed to fetch all tags:', error);
        }
    },

    setViewMode: (mode) => set({ viewMode: mode }),

    init: () => {
        if (get().isInitialized) {
            return;
        }
        set({ isInitialized: true });

        useAuthStore.subscribe(
            (state, prevState) => {
                if (state.isAuthenticated !== prevState.isAuthenticated) {
                    get()._initializeWebSocket(state.isAuthenticated);
                }
            }
        );

        const initialIsAuthenticated = useAuthStore.getState().isAuthenticated;
        get()._initializeWebSocket(initialIsAuthenticated);
    },

    _initializeWebSocket: (isAuthenticated) => {
        const { connectionStatus } = get();
        if (connectionStatus === 'connected' && useAuthStore.getState().isAuthenticated === isAuthenticated) {
            return;
        }

        set({ connectionStatus: 'connecting', isLoading: true, error: null });
        get().disconnectWebSocket();

        const token = useAuthStore.getState().token;
        if (isAuthenticated && !token) {
            set({ connectionStatus: 'error', error: 'Authentication token not found.', isLoading: false });
            return;
        }

        websocketService.on('open', get()._handleWebSocketOpen);
        websocketService.on('full_server_list', get()._handleWebSocketMessage);
        websocketService.on('service_monitor_result', get()._handleServiceMonitorResult);
        websocketService.on('performance_metric_batch', get()._handlePerformanceMetricBatch);
        websocketService.on('close', get()._handleWebSocketClose);
        websocketService.on('error', get()._handleWebSocketError);
        websocketService.on('permanent_failure', get()._handleWebSocketPermanentFailure);
        
        websocketService.connect(isAuthenticated ? token! : undefined);
    },

    disconnectWebSocket: () => {
      console.log('ServerListStore: Disconnecting WebSocket and removing listeners.');
      
      websocketService.disconnect();

      websocketService.off('open', get()._handleWebSocketOpen);
      websocketService.off('full_server_list', get()._handleWebSocketMessage);
      websocketService.off('service_monitor_result', get()._handleServiceMonitorResult);
      websocketService.off('performance_metric_batch', get()._handlePerformanceMetricBatch);
      websocketService.off('close', get()._handleWebSocketClose);
      websocketService.off('error', get()._handleWebSocketError);
      websocketService.off('permanent_failure', get()._handleWebSocketPermanentFailure);
      
      get().clearVpsMonitorResults();
      get().clearMonitorResults();
      get().clearVpsPerformanceMetrics();

      set({ connectionStatus: 'disconnected' });
    },

    _handleWebSocketOpen: () => {
        console.log('ServerListStore: WebSocket connection opened.');
        set({ connectionStatus: 'connected', error: null });
    },

    _handleWebSocketMessage: (data) => {
        const newServers = data.servers;
        const oldServers = get().servers;

        if (!equal(oldServers, newServers)) {
            set({ servers: newServers, isLoading: false, connectionStatus: 'connected', error: null });
        } else if (get().isLoading || get().connectionStatus !== 'connected') {
             set({ isLoading: false, connectionStatus: 'connected', error: null });
        }
    },

    _handleServiceMonitorResult: (data) => {
        const vpsId = data.agentId;
        if (vpsId === undefined) return;
        
        // --- Handle Per-VPS Pub/Sub (for HomePage) ---
        if (vpsMonitorResultListeners[vpsId]) {
            // Also update the cache so re-mounts get the latest data
            if (vpsMonitorResultsCache[vpsId]) {
                vpsMonitorResultsCache[vpsId].push(data);
                const now = Date.now();
                const windowStartTime = now - (10 * 60 * 1000); // 10 minute window
                const filteredCache = vpsMonitorResultsCache[vpsId].filter(
                    p => new Date(p.time).getTime() >= windowStartTime
                );
                vpsMonitorResultsCache[vpsId] = filteredCache;
            }
            // Push the entire filtered cache to subscribers, not just the new point
            vpsMonitorResultListeners[vpsId].forEach(callback => callback(vpsMonitorResultsCache[vpsId]));
        }

        // --- Handle Per-Monitor Pub/Sub (for ServiceMonitorDetailPage) ---
        const { monitorId } = data;
        if (monitorResultListeners[monitorId]) {
            if (!monitorResultsCache[monitorId]) monitorResultsCache[monitorId] = [];
            monitorResultsCache[monitorId].push(data);
            const CACHE_MAX_SIZE = 1000;
            if (monitorResultsCache[monitorId].length > CACHE_MAX_SIZE) {
                monitorResultsCache[monitorId] = monitorResultsCache[monitorId].slice(-CACHE_MAX_SIZE);
            }
            monitorResultListeners[monitorId].forEach(callback => callback([data]));
        }
    },

    _handlePerformanceMetricBatch: (data: PerformanceMetricBatch) => {
        const { metrics } = data;
        if (!metrics || metrics.length === 0) return;

        const metricsByVps = metrics.reduce((acc, metric) => {
            if (!acc[metric.vpsId]) acc[metric.vpsId] = [];
            acc[metric.vpsId].push(metric);
            return acc;
        }, {} as Record<number, PerformanceMetricPoint[]>);

        // Update latestMetrics for immediate UI feedback (e.g., stat cards)
        set(state => {
            const newLatestMetrics = { ...state.latestMetrics };
            for (const vpsId in metricsByVps) {
                const vpsMetrics = metricsByVps[vpsId];
                if (vpsMetrics.length > 0) {
                    const latestMetricInBatch = vpsMetrics.reduce((latest, current) => 
                        new Date(current.time) > new Date(latest.time) ? current : latest
                    );
                    newLatestMetrics[vpsId] = latestMetricInBatch;
                }
            }
            return { latestMetrics: newLatestMetrics };
        });

        // Append new metrics to the historical/real-time data array
        set(state => {
            const newInitialMetrics = { ...state.initialVpsMetrics };
            for (const vpsIdStr in metricsByVps) {
                const vpsId = parseInt(vpsIdStr, 10);
                if (newInitialMetrics[vpsId]?.status === 'success') {
                    const newPoints = metricsByVps[vpsId];
                    const existingPoints = newInitialMetrics[vpsId].data;
                    const combinedPoints = [...existingPoints, ...newPoints];
                    const now = Date.now();
                    const windowStartTime = now - (10 * 60 * 1000); // 10 minute window

                    const filteredPoints = combinedPoints.filter(
                        p => new Date(p.time).getTime() >= windowStartTime
                    );

                    newInitialMetrics[vpsId] = {
                        ...newInitialMetrics[vpsId],
                        data: filteredPoints,
                    };
                }
            }
            return { initialVpsMetrics: newInitialMetrics };
        });
    },

    _handleWebSocketClose: ({ isIntentional, event }) => {
        console.log(`ServerListStore: WebSocket connection closed. Intentional: ${isIntentional}, Code: ${event.code}`);
        if (isIntentional) {
            set({ connectionStatus: 'disconnected', isLoading: false });
        } else if (get().connectionStatus !== 'permanently_failed') {
             set({ connectionStatus: 'reconnecting', isLoading: true, error: `Connection closed (code: ${event.code}). Attempting to reconnect.` });
        }
    },

    _handleWebSocketError: (event: Event) => {
        console.error('ServerListStore: WebSocket error.', event);
        if (get().connectionStatus !== 'reconnecting' && get().connectionStatus !== 'permanently_failed') {
            set({ connectionStatus: 'error', error: 'WebSocket connection error.', isLoading: false });
        } else if (get().connectionStatus === 'reconnecting') {
             set({ error: 'Error during reconnection attempt.' });
        }
    },

    _handleWebSocketPermanentFailure: () => {
        console.error('ServerListStore: WebSocket permanent failure after multiple retries.');
        set({
            connectionStatus: 'permanently_failed',
            error: 'Failed to connect to WebSocket after multiple retries.',
            isLoading: false,
        });
    },
    }),
    {
      name: 'server-list-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ viewMode: state.viewMode }),
    }
  )
);