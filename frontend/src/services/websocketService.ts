import { useAuthStore } from '../store/authStore';
import { EventEmitter } from './eventEmitter';
import type { FullServerListPushType, ServiceMonitorResult } from '../types';
import { throttle } from 'lodash';

const WS_URL_BASE = import.meta.env.VITE_WS_BASE_URL || `ws://${window.location.host}`;

// Define the events and their payload types
interface WebSocketEvents {
  open: void;
  close: { isIntentional: boolean; event: CloseEvent };
  error: Event;
  permanent_failure: void;
  full_server_list: FullServerListPushType;
  service_monitor_result: ServiceMonitorResult;
  // Add other specific message types here
}

class WebSocketService extends EventEmitter<WebSocketEvents> {
    private ws: WebSocket | null = null;
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectTimeoutId: number | null = null;
    private intentionalClose = false;
    private currentToken: string | null = null;

    private throttledEmitFullServerList: (data: FullServerListPushType) => void;

    constructor() {
        super();
        this.throttledEmitFullServerList = throttle((data: FullServerListPushType) => {
            this.emit('full_server_list', data);
        }, 2000, { leading: true, trailing: true }); // Throttle to once every 2 seconds
    }

    private getWebSocketUrl(token: string): string {
        const url = new URL('/ws/metrics', WS_URL_BASE);
        url.searchParams.append('token', token);
        if (url.protocol === 'http:') {
            url.protocol = 'ws:';
        } else if (url.protocol === 'https:') {
            url.protocol = 'wss:';
        }
        return url.toString();
    }

    public connect(token: string): void {
        if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
            console.log('WebSocket is already connected or connecting.');
            return;
        }

        this.currentToken = token;
        this.intentionalClose = false;

        if (!this.currentToken) {
            console.error('WebSocket connection attempt without a token.');
            this.emit('error', new Event('No token provided'));
            return;
        }

        const wsUrl = this.getWebSocketUrl(this.currentToken);
        console.log('Attempting to connect to WebSocket:', wsUrl);
        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            console.log('WebSocket connection established.');
            this.reconnectAttempts = 0;
            if (this.reconnectTimeoutId) {
                clearTimeout(this.reconnectTimeoutId);
                this.reconnectTimeoutId = null;
            }
            this.emit('open', undefined);
        };

        this.ws.onmessage = (event) => {
            let parsedData;
            try {
                parsedData = JSON.parse(event.data as string);
            } catch (error) {
                console.error('WebSocketService: Error parsing message JSON:', error);
                return;
            }

            if (parsedData && typeof parsedData === 'object') {
                // Case 1: Message has a 'type' field (structured messages)
                if ('type' in parsedData) {
                    switch (parsedData.type) {
                        case 'ping':
                            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                                this.ws.send(JSON.stringify({ type: 'pong' }));
                            }
                            return;
                        case 'connected':
                            console.log('WebSocketService: Received "connected" confirmation.');
                            return;
                        case 'service_monitor_result':
                            this.emit('service_monitor_result', parsedData.data as ServiceMonitorResult);
                            return;
                        // Note: 'full_server_list' might not be used if the raw object is sent instead
                        case 'full_server_list':
                             this.throttledEmitFullServerList(parsedData.data as FullServerListPushType);
                             return;
                        default:
                            console.warn('WebSocketService: Received unknown message type:', parsedData.type);
                            return;
                    }
                }

                // Case 2: Raw server list push (for backward compatibility or other push types)
                if ('servers' in parsedData && Array.isArray(parsedData.servers)) {
                    this.throttledEmitFullServerList(parsedData as FullServerListPushType);
                    return;
                }

                // If neither condition is met, it's a malformed message
                console.warn('WebSocketService: Received malformed message:', parsedData);

            } else {
                 console.error('WebSocketService: Received message that is not a JSON object or is null.');
            }
        };

        this.ws.onclose = (event) => {
            console.log(`WebSocket connection closed. Code: ${event.code}, Intentional: ${this.intentionalClose}`);
            this.emit('close', { isIntentional: this.intentionalClose, event });
            if (!this.intentionalClose) {
                this.handleReconnect();
            }
        };

        this.ws.onerror = (event) => {
            console.error('WebSocket error:', event);
            this.emit('error', event);
        };
    }

    private handleReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('WebSocket: Maximum reconnect attempts reached.');
            this.emit('permanent_failure', undefined);
            return;
        }

        this.reconnectAttempts++;
        const delay = Math.min(30000, (2 ** this.reconnectAttempts) * 1000);
        console.log(`WebSocket: Reconnecting in ${delay / 1000}s (attempt ${this.reconnectAttempts})`);

        if (this.reconnectTimeoutId) clearTimeout(this.reconnectTimeoutId);

        this.reconnectTimeoutId = window.setTimeout(() => {
            const token = useAuthStore.getState().token;
            if (token) {
                this.connect(token);
            } else {
                console.error('WebSocket: No token for reconnect.');
                this.emit('permanent_failure', undefined);
            }
        }, delay);
    }

    public disconnect(): void {
        console.log('WebSocket: Disconnecting intentionally.');
        this.intentionalClose = true;
        if (this.reconnectTimeoutId) {
            clearTimeout(this.reconnectTimeoutId);
            this.reconnectTimeoutId = null;
        }
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.reconnectAttempts = 0;
    }

    public send(message: object): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        } else {
            console.error('WebSocket is not connected. Cannot send message.');
        }
    }
}

// Export a singleton instance
const websocketService = new WebSocketService();
export default websocketService;