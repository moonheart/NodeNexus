import { create } from 'zustand';
import WebSocketService from '../services/websocketService';
import { useAuthStore } from './authStore';
import type { VpsListItemResponse, FullServerListPushType } from '../types';

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
    websocketService: WebSocketService | null;
    initializeWebSocket: () => void;
    disconnectWebSocket: () => void;
    // Internal actions to be called by WebSocketService callbacks
    _handleWebSocketOpen: () => void;
    _handleWebSocketMessage: (data: FullServerListPushType) => void;
    _handleWebSocketClose: (isIntentional: boolean, event: CloseEvent) => void;
    _handleWebSocketError: (event: Event) => void;
    _handleWebSocketPermanentFailure: () => void;
}

export const useServerListStore = create<ServerListState>((set, get) => ({
    servers: [],
    connectionStatus: 'disconnected',
    isLoading: true, // Assume loading initially until first connection or message
    error: null,
    websocketService: null,

    initializeWebSocket: () => {
        const { websocketService: currentService, connectionStatus } = get();

        // If already connected or trying to connect/reconnect, do nothing.
        if (connectionStatus === 'connected' || connectionStatus === 'connecting' || connectionStatus === 'reconnecting') {
            console.log(`ServerListStore: WebSocket already ${connectionStatus}. Initialization skipped.`);
            return;
        }

        // If there's an existing service instance (e.g., from a failed/closed/error state),
        // ensure it's properly disconnected before creating a new one.
        // The disconnectWebSocket action also nullifies the service in the store.
        if (currentService) {
            console.log('ServerListStore: Previous WebSocket service instance found. Disconnecting it.');
            get().disconnectWebSocket(); // This will call currentService.disconnect() and set store's service to null
        }

        const token = useAuthStore.getState().token;
        if (!token) {
            console.error('ServerListStore: Cannot initialize WebSocket: No auth token found.');
            set({ connectionStatus: 'error', error: 'Authentication token not found.', isLoading: false });
            return;
        }

        set({ connectionStatus: 'connecting', isLoading: true, error: null });

        const newService = new WebSocketService({
            onOpen: () => get()._handleWebSocketOpen(),
            onMessage: (data) => get()._handleWebSocketMessage(data),
            onClose: (isIntentional, event) => get()._handleWebSocketClose(isIntentional, event),
            onError: (event) => get()._handleWebSocketError(event),
            onPermanentFailure: () => get()._handleWebSocketPermanentFailure(),
        });

        set({ websocketService: newService }); // Store the new service instance
        newService.connect(token);
    },

    disconnectWebSocket: () => {
        const { websocketService } = get();
        if (websocketService) {
            console.log('ServerListStore: Disconnecting WebSocket.');
            websocketService.disconnect();
        }
        // State update for intentional disconnect is handled by _handleWebSocketClose
        // We can also set it here if immediate feedback is desired before ws.onclose fires
        // set({ connectionStatus: 'disconnected', isLoading: false }); // Optional: immediate feedback
        set({ websocketService: null }); // Clear the service instance
    },

    _handleWebSocketOpen: () => {
        console.log('ServerListStore: WebSocket connection opened.');
        set({ connectionStatus: 'connected', isLoading: false, error: null });
    },

    _handleWebSocketMessage: (data: FullServerListPushType) => {
        // console.log('ServerListStore: Received WebSocket message:', data);
        set({ servers: data.servers, isLoading: false, error: null }); // Assuming full list push
    },

    _handleWebSocketClose: (isIntentional: boolean, event: CloseEvent) => {
        console.log(`ServerListStore: WebSocket connection closed. Intentional: ${isIntentional}, Code: ${event.code}`);
        if (isIntentional) {
            set({ connectionStatus: 'disconnected', isLoading: false });
        } else {
            // Reconnect attempts are handled by WebSocketService,
            // it will call onOpen if successful, or onPermanentFailure if not.
            // We can set status to 'reconnecting' here if WebSocketService doesn't manage this state itself via callbacks.
            // For now, assuming WebSocketService's onError or onPermanentFailure will update status.
            // If no specific "reconnecting" state is signaled by the service, we might just see 'disconnected' then 'connecting'
            // or 'error' if an error occurs during reconnect.
            // Let's assume the service will trigger onError or onOpen during its attempts.
            // If it just closes and tries again, this 'disconnected' state is accurate until next 'connecting' or 'open'.
            if (get().connectionStatus !== 'permanently_failed') { // Avoid overriding permanent failure
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
}));

// Optional: Expose a way to get the service instance if needed for direct calls, though usually not recommended.
// export const getWebsocketServiceInstance = () => useServerListStore.getState().websocketService;