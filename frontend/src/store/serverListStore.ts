import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import websocketService from '../services/websocketService';
import { useAuthStore } from './authStore';
import type { VpsListItemResponse, FullServerListPushType, ViewMode, Tag, ServiceMonitorResult } from '../types';
import * as tagService from '../services/tagService';
import equal from 'fast-deep-equal';
import { getMonitorResults, getMonitorResultsByVpsId } from '../services/serviceMonitorService';

// --- Service Monitor Pub/Sub System (Per VPS) ---
// This system lives outside the Zustand state to avoid causing re-renders on data updates.
// Components will subscribe to updates for a specific VPS and manage their own state.

type VpsMonitorResultCallback = (results: ServiceMonitorResult[]) => void;
type MonitorResultCallback = (results: ServiceMonitorResult[]) => void; // For monitor-specific page
export type UnsubscribeFunction = () => void;

// Cache to hold monitor results. Key is vpsId.
const vpsMonitorResultsCache: Record<number, ServiceMonitorResult[]> = {};
// Listeners for monitor updates. Key is vpsId.
const vpsMonitorResultListeners: Record<number, Set<VpsMonitorResultCallback>> = {};
const isFetchingInitialVpsData: Record<number, boolean> = {};

// --- Legacy Pub/Sub for ServiceMonitorDetailPage ---
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

export interface ServerListState { // Added export
    servers: VpsListItemResponse[];
    connectionStatus: ConnectionStatus;
    isLoading: boolean; // For initial load or when explicitly loading/reconnecting
    error: string | null; // For WebSocket related errors
    viewMode: ViewMode;
    allTags: Tag[]; // To store all available tags globally
    isInitialized: boolean; // Flag to prevent double initialization
    fetchAllTags: () => Promise<void>; // Action to fetch all tags
    setViewMode: (mode: ViewMode) => void;
    init: () => void; // New action to start listening to auth changes
    disconnectWebSocket: () => void;

    // Service Monitor Pub/Sub Actions (per VPS for homepage)
    subscribeToVpsMonitorResults: (vpsId: number, callback: VpsMonitorResultCallback) => UnsubscribeFunction;
    getInitialVpsMonitorResults: (vpsId: number, limit?: number) => Promise<ServiceMonitorResult[]>;
    clearVpsMonitorResults: () => void;

    // Service Monitor Pub/Sub Actions (per Monitor for detail page)
    subscribeToMonitorResults: (monitorId: number, callback: MonitorResultCallback) => UnsubscribeFunction;
    getInitialMonitorResults: (monitorId: number, limit?: number) => Promise<ServiceMonitorResult[]>;
    clearMonitorResults: () => void;

    // Internal actions
    _initializeWebSocket: (isAuthenticated: boolean) => void;
    _handleWebSocketOpen: () => void;
    _handleWebSocketMessage: (data: FullServerListPushType) => void;
    _handleServiceMonitorResult: (data: ServiceMonitorResult) => void;
    _handleWebSocketClose: (data: { isIntentional: boolean; event: CloseEvent }) => void;
    _handleWebSocketError: (event: Event) => void;
    _handleWebSocketPermanentFailure: () => void;
}

export const useServerListStore = create<ServerListState>()(
  persist(
    (set, get) => ({
      servers: [],
      connectionStatus: 'disconnected',
      isLoading: true, // Assume loading initially until first connection or message
      error: null,
      viewMode: 'card', // Default view mode
      allTags: [],
      isInitialized: false,

    // --- Service Monitor Pub/Sub Implementation (Per VPS) ---
    subscribeToVpsMonitorResults: (vpsId, callback) => {
        if (!vpsMonitorResultListeners[vpsId]) {
            vpsMonitorResultListeners[vpsId] = new Set();
        }
        vpsMonitorResultListeners[vpsId].add(callback);

        // Return an unsubscribe function
        return () => {
            vpsMonitorResultListeners[vpsId]?.delete(callback);
            if (vpsMonitorResultListeners[vpsId]?.size === 0) {
                delete vpsMonitorResultListeners[vpsId];
                // Optional: also clear cache for this vpsId if no longer needed
                // delete vpsMonitorResultsCache[vpsId];
            }
        };
    },

    getInitialVpsMonitorResults: async (vpsId, limit = 500) => {
        if (vpsMonitorResultsCache[vpsId]) {
            return vpsMonitorResultsCache[vpsId];
        }

        if (isFetchingInitialVpsData[vpsId]) {
            // Avoid race conditions where multiple components request the same data simultaneously
            await new Promise(resolve => setTimeout(resolve, 100)); // simple wait
            return get().getInitialVpsMonitorResults(vpsId, limit); // retry
        }

        try {
            isFetchingInitialVpsData[vpsId] = true;
            // Use the new service function to fetch by VPS ID
            const results = await getMonitorResultsByVpsId(vpsId, undefined, undefined, limit);
            const sortedResults = results.sort((a, b) => new Date(a.time).getTime() - new Date(b.time).getTime());
            vpsMonitorResultsCache[vpsId] = sortedResults;
            return sortedResults;
        } catch (error) {
            console.error(`Failed to get initial monitor results for VPS ${vpsId}:`, error);
            return []; // Return empty array on error
        } finally {
            isFetchingInitialVpsData[vpsId] = false;
        }
    },
    
    clearVpsMonitorResults: () => {
        Object.keys(vpsMonitorResultsCache).forEach(key => delete vpsMonitorResultsCache[Number(key)]);
        Object.keys(vpsMonitorResultListeners).forEach(key => delete vpsMonitorResultListeners[Number(key)]);
        console.log('Cleared all VPS service monitor caches and listeners.');
    },

    // --- Legacy Pub/Sub Implementation for ServiceMonitorDetailPage ---
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

    getInitialMonitorResults: async (monitorId, limit = 500) => {
        if (monitorResultsCache[monitorId]) {
            return monitorResultsCache[monitorId];
        }
        if (isFetchingInitialData[monitorId]) {
            await new Promise(resolve => setTimeout(resolve, 100));
            return get().getInitialMonitorResults(monitorId, limit);
        }
        try {
            isFetchingInitialData[monitorId] = true;
            const results = await getMonitorResults(monitorId, undefined, undefined, limit);
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
            // Optionally handle the error in the UI
        }
    },

    setViewMode: (mode) => set({ viewMode: mode }),

    init: () => {
        // Make the init function idempotent to prevent issues with React's Strict Mode.
        if (get().isInitialized) {
            console.log("ServerListStore: Already initialized.");
            return;
        }
        set({ isInitialized: true });

        console.log("ServerListStore: Initializing and subscribing to auth changes.");

        // Subscribe to the auth store
        useAuthStore.subscribe(
            (state, prevState) => {
                if (state.isAuthenticated !== prevState.isAuthenticated) {
                    console.log(`ServerListStore: Detected auth state change from ${prevState.isAuthenticated} to ${state.isAuthenticated}.`);
                    get()._initializeWebSocket(state.isAuthenticated);
                }
            }
        );

        // Initial connection based on the current state
        const initialIsAuthenticated = useAuthStore.getState().isAuthenticated;
        get()._initializeWebSocket(initialIsAuthenticated);
    },

    _initializeWebSocket: (isAuthenticated) => {
        const { connectionStatus } = get();
        // We still check status to avoid redundant connections if state somehow doesn't change.
        if (connectionStatus === 'connected' && useAuthStore.getState().isAuthenticated === isAuthenticated) {
            console.log('Connection already established with correct auth state.');
            return;
        }

        set({ connectionStatus: 'connecting', isLoading: true, error: null });

        // Centralized disconnect before reconnecting
        get().disconnectWebSocket();

        if (isAuthenticated) {
            const token = useAuthStore.getState().token;
            if (!token) {
                set({ connectionStatus: 'error', error: 'Authentication token not found.', isLoading: false });
                return;
            }
            // Register event listeners for the private service
            websocketService.on('open', get()._handleWebSocketOpen);
            websocketService.on('full_server_list', get()._handleWebSocketMessage);
            websocketService.on('service_monitor_result', get()._handleServiceMonitorResult);
            websocketService.on('close', get()._handleWebSocketClose);
            websocketService.on('error', get()._handleWebSocketError);
            websocketService.on('permanent_failure', get()._handleWebSocketPermanentFailure);
            websocketService.connect(token);
        } else {
            // Register event listeners for the public service (now handled by the same service)
            websocketService.on('open', get()._handleWebSocketOpen);
            websocketService.on('full_server_list', get()._handleWebSocketMessage);
            // No service_monitor_result for public view
            websocketService.on('close', get()._handleWebSocketClose);
            websocketService.on('error', get()._handleWebSocketError);
            websocketService.on('permanent_failure', get()._handleWebSocketPermanentFailure);
            websocketService.connect(); // Connect without a token for the public endpoint
        }
    },

    disconnectWebSocket: () => {
      console.log('ServerListStore: Disconnecting WebSocket and removing listeners.');
      
      // Disconnect the single service
      websocketService.disconnect();

      // Unregister all event listeners to prevent memory leaks
      websocketService.off('open', get()._handleWebSocketOpen);
      websocketService.off('full_server_list', get()._handleWebSocketMessage);
      websocketService.off('service_monitor_result', get()._handleServiceMonitorResult);
      websocketService.off('close', get()._handleWebSocketClose);
      websocketService.off('error', get()._handleWebSocketError);
      websocketService.off('permanent_failure', get()._handleWebSocketPermanentFailure);
      
      // Clean up monitor data on disconnect
      get().clearVpsMonitorResults();
      get().clearMonitorResults();

      set({ connectionStatus: 'disconnected' });
    },

    _handleWebSocketOpen: () => {
        console.log('ServerListStore: WebSocket connection opened.');
        set({ connectionStatus: 'connected', error: null }); // Keep isLoading: true until the first message arrives
    },

    _handleWebSocketMessage: (data) => {
        const newServers = data.servers;
        const oldServers = get().servers;

        // Create a map of new servers for efficient lookup
        const newServersMap = new Map(newServers.map(s => [s.id, s]));

        // Create a new array, reusing old server objects if they haven't changed.
        const mergedServers = newServers.map(newServer => {
            const oldServer = oldServers.find(s => s.id === newServer.id);
            // If an old server exists and is deep-equal to the new one, reuse the old object reference
            if (oldServer && equal(oldServer, newServer)) {
                return oldServer;
            }
            // Otherwise, use the new server object
            return newServer;
        });

        // Also, handle servers that might have been removed.
        const finalServers = mergedServers.filter(s => newServersMap.has(s.id));
        
        // Check if the final array is different from the old one before setting state
        if (!equal(oldServers, finalServers)) {
            set({ servers: finalServers, isLoading: false, connectionStatus: 'connected', error: null });
        } else {
            // If nothing changed, we can avoid a state update entirely.
            // But we still need to update loading/connection status on the first message.
            if (get().isLoading || get().connectionStatus !== 'connected') {
                 set({ isLoading: false, connectionStatus: 'connected', error: null });
            }
        }
    },

    _handleServiceMonitorResult: (data) => {
        // Use agentId as the key for routing, as per user feedback.
        const vpsId = data.agentId;

        if (vpsId === undefined) {
            console.warn('Received service monitor result without an agentId. Ignoring.', data);
            return;
        }
        
        // Update cache for the specific VPS (keyed by agentId)
        if (!vpsMonitorResultsCache[vpsId]) {
            vpsMonitorResultsCache[vpsId] = [];
        }
        vpsMonitorResultsCache[vpsId].push(data);

        // Keep cache size reasonable
        const CACHE_MAX_SIZE = 1000;
        if (vpsMonitorResultsCache[vpsId].length > CACHE_MAX_SIZE) {
            vpsMonitorResultsCache[vpsId] = vpsMonitorResultsCache[vpsId].slice(-CACHE_MAX_SIZE);
        }

        // Notify listeners subscribed to this specific vpsId (agentId)
        if (vpsMonitorResultListeners[vpsId]) {
            vpsMonitorResultListeners[vpsId].forEach(callback => {
                callback([data]); // Pass new result as an array
            });
        }

        // Also notify legacy listeners for the detail page (keyed by monitorId)
        const { monitorId } = data;
        if (monitorResultListeners[monitorId]) {
            if (!monitorResultsCache[monitorId]) {
                monitorResultsCache[monitorId] = [];
            }
            monitorResultsCache[monitorId].push(data);
            if (monitorResultsCache[monitorId].length > CACHE_MAX_SIZE) {
                monitorResultsCache[monitorId] = monitorResultsCache[monitorId].slice(-CACHE_MAX_SIZE);
            }
            monitorResultListeners[monitorId].forEach(callback => {
                // Legacy system now also gets only the new result, just like the new system.
                // The component is now responsible for accumulating the results.
                callback([data]);
            });
        }
    },

    _handleWebSocketClose: ({ isIntentional, event }) => {
        console.log(`ServerListStore: WebSocket connection closed. Intentional: ${isIntentional}, Code: ${event.code}`);
        if (isIntentional) {
            set({ connectionStatus: 'disconnected', isLoading: false });
        } else {
            if (get().connectionStatus !== 'permanently_failed') {
                 set({ connectionStatus: 'reconnecting', isLoading: true, error: `Connection closed (code: ${event.code}). Attempting to reconnect.` });
            }
        }
    },

    _handleWebSocketError: (event: Event) => {
        console.error('ServerListStore: WebSocket error.', event);
        // Avoid setting isLoading to false if we are in 'reconnecting' state
        // and an error occurs during a reconnect attempt. The service will handle further retries.
        if (get().connectionStatus !== 'reconnecting' && get().connectionStatus !== 'permanently_failed') {
            set({ connectionStatus: 'error', error: 'WebSocket connection error.', isLoading: false });
        } else if (get().connectionStatus === 'reconnecting') {
             set({ error: 'Error during reconnection attempt.' }); // Keep status as 'reconnecting'
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
      name: 'server-list-storage', // unique name
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ viewMode: state.viewMode }), // Only persist the viewMode
    }
  )
);

// Optional: Expose a way to get the service instance if needed for direct calls, though usually not recommended.
// export const getWebsocketServiceInstance = () => useServerListStore.getState().websocketService;