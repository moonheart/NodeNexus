import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import websocketService from '../services/websocketService';
import { useAuthStore } from './authStore';
import type { VpsListItemResponse, FullServerListPushType, ViewMode, Tag, ServiceMonitorResult } from '../types';
import * as tagService from '../services/tagService';

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

      set({ connectionStatus: 'disconnected' });
    },

    _handleWebSocketOpen: () => {
        console.log('ServerListStore: WebSocket connection opened.');
        set({ connectionStatus: 'connected', error: null }); // Keep isLoading: true until the first message arrives
    },

    _handleWebSocketMessage: (data) => {
        // For both authenticated and public views, a simple replacement is the most
        // reliable way to handle additions, updates, and deletions from the server.
        // The server is the source of truth.
        set({ servers: data.servers, isLoading: false, connectionStatus: 'connected', error: null });
    },

    _handleServiceMonitorResult: (data) => {
        // This is handled by the detail page, but we could add logic here if needed globally.
        console.log('Received service monitor result in store:', data.monitorId);
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