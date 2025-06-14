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
    fetchAllTags: () => Promise<void>; // Action to fetch all tags
    setViewMode: (mode: ViewMode) => void;
    initializeWebSocket: () => void;
    disconnectWebSocket: () => void;
    // Internal actions to be called by WebSocketService callbacks
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

    initializeWebSocket: () => {
      const { connectionStatus } = get();
      if (connectionStatus === 'connected' || connectionStatus === 'connecting' || connectionStatus === 'reconnecting') {
        return;
      }

      const token = useAuthStore.getState().token;
      if (!token) {
        set({ connectionStatus: 'error', error: 'Authentication token not found.', isLoading: false });
        return;
      }

      set({ connectionStatus: 'connecting', isLoading: true, error: null });

      // Register event listeners
      websocketService.on('open', get()._handleWebSocketOpen);
      websocketService.on('full_server_list', get()._handleWebSocketMessage);
      websocketService.on('service_monitor_result', get()._handleServiceMonitorResult);
      websocketService.on('close', get()._handleWebSocketClose);
      websocketService.on('error', get()._handleWebSocketError);
      websocketService.on('permanent_failure', get()._handleWebSocketPermanentFailure);

      websocketService.connect(token);
    },

    disconnectWebSocket: () => {
      console.log('ServerListStore: Disconnecting WebSocket.');
      websocketService.disconnect();

      // Unregister event listeners
      websocketService.off('open', get()._handleWebSocketOpen);
      websocketService.off('full_server_list', get()._handleWebSocketMessage);
      websocketService.off('service_monitor_result', get()._handleServiceMonitorResult);
      websocketService.off('close', get()._handleWebSocketClose);
      websocketService.off('error', get()._handleWebSocketError);
      websocketService.off('permanent_failure', get()._handleWebSocketPermanentFailure);
    },

    _handleWebSocketOpen: () => {
        console.log('ServerListStore: WebSocket connection opened.');
        set({ connectionStatus: 'connected', isLoading: false, error: null });
    },

    _handleWebSocketMessage: (data) => {
        set({ servers: data.servers, isLoading: false, error: null, connectionStatus: 'connected' });
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