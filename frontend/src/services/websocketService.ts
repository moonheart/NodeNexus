import { useAuthStore } from '../store/authStore';
import type { FullServerListPushType } from '../types'; // Assuming this will be defined

const WS_URL_BASE = import.meta.env.VITE_WS_BASE_URL || `ws://${window.location.host}`;

interface WebSocketServiceOptions {
    onOpen: () => void;
    onMessage: (data: FullServerListPushType) => void;
    onClose: (isIntentional: boolean, event: CloseEvent) => void;
    onError: (event: Event) => void;
    onPermanentFailure?: () => void; // Optional: if all retries fail
}

class WebSocketService {
    private ws: WebSocket | null = null;
    private options: WebSocketServiceOptions;
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5; // Max 5 retries
    private reconnectTimeoutId: number | null = null;
    private intentionalClose = false;
    private currentToken: string | null = null;

    constructor(options: WebSocketServiceOptions) {
        this.options = options;
    }

    private getWebSocketUrl(token: string): string {
        const url = new URL('/ws/metrics', WS_URL_BASE);
        url.searchParams.append('token', token);
        // Ensure ws:// or wss://
        if (url.protocol === 'http:') {
            url.protocol = 'ws:';
        } else if (url.protocol === 'https:') {
            url.protocol = 'wss:';
        }
        return url.toString();
    }

    public connect(token: string): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            console.log('WebSocket is already connected.');
            return;
        }
        if (this.ws && this.ws.readyState === WebSocket.CONNECTING) {
            console.log('WebSocket is already connecting.');
            return;
        }

        this.currentToken = token;
        this.intentionalClose = false; // Reset flag on new connect attempt

        if (!this.currentToken) {
            console.error('WebSocket connection attempt without a token.');
            // Optionally, notify the store about this specific error
            this.options.onError(new Event('No token provided for WebSocket connection'));
            return;
        }

        const wsUrl = this.getWebSocketUrl(this.currentToken);
        console.log('Attempting to connect to WebSocket:', wsUrl);
        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            console.log('WebSocket connection established.');
            this.reconnectAttempts = 0; // Reset on successful connection
            if (this.reconnectTimeoutId) {
                clearTimeout(this.reconnectTimeoutId);
                this.reconnectTimeoutId = null;
            }
            this.options.onOpen();
        };

        this.ws.onmessage = (event) => {
            let parsedData;
            try {
                parsedData = JSON.parse(event.data as string);
            } catch (error) {
                console.error('WebSocketService: Error parsing WebSocket message JSON:', error, 'Raw data:', event.data);
                return; // Cannot process malformed JSON
            }

            if (parsedData && typeof parsedData === 'object') {
                // Handle server-sent ping
                if (parsedData.type === 'ping') {
                    console.log('WebSocketService: Received ping from server, sending pong.');
                    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                        this.ws.send(JSON.stringify({ type: 'pong' }));
                    }
                    return; // Ping handled, do not pass to application logic
                }

                // Handle server-sent connected confirmation
                if (parsedData.type === 'connected') {
                    console.log('WebSocketService: Received "connected" message from server.');
                    // This type of message is usually for confirmation and not directly fed into data stores
                    // unless a specific state needs to be set based on it.
                    // For now, we just log it and don't pass it to this.options.onMessage
                    // as onMessage expects FullServerListPushType.
                    return; // "connected" message handled (logged)
                }

                // Check if it's likely a FullServerListPushType message
                // This assumes FullServerListPushType is an object and always contains a 'servers' array.
                if (Array.isArray(parsedData.servers)) {
                    this.options.onMessage(parsedData as FullServerListPushType);
                } else {
                    console.warn('WebSocketService: Received unknown JSON message structure:', parsedData);
                }
            } else {
                console.warn('WebSocketService: Received message that is not a JSON object or is null:', parsedData);
            }
        };

        this.ws.onclose = (event) => {
            console.log(`WebSocket connection closed. Code: ${event.code}, Reason: '${event.reason}', WasClean: ${event.wasClean}, Intentional: ${this.intentionalClose}`);
            this.options.onClose(this.intentionalClose, event);
            if (!this.intentionalClose) {
                this.handleReconnect();
            }
        };

        this.ws.onerror = (event) => {
            console.error('WebSocket error:', event);
            this.options.onError(event);
            // Note: onerror is often followed by onclose. Reconnect logic is in onclose.
        };
    }

    private handleReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('WebSocket: Maximum reconnect attempts reached.');
            if (this.options.onPermanentFailure) {
                this.options.onPermanentFailure();
            }
            return;
        }

        this.reconnectAttempts++;
        const delay = Math.min(30000, (2 ** this.reconnectAttempts) * 1000); // Exponential backoff, max 30s
        console.log(`WebSocket: Attempting to reconnect in ${delay / 1000}s (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts}).`);

        if (this.reconnectTimeoutId) {
            clearTimeout(this.reconnectTimeoutId);
        }

        this.reconnectTimeoutId = window.setTimeout(() => {
            const token = useAuthStore.getState().token; // Get fresh token
            if (token) {
                this.connect(token);
            } else {
                console.error('WebSocket: No token available for reconnect attempt.');
                if (this.options.onPermanentFailure) { // Or a different callback for auth failure
                    this.options.onPermanentFailure();
                }
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
        this.reconnectAttempts = 0; // Reset attempts on intentional disconnect
    }

    public send(message: string | ArrayBufferLike | Blob | ArrayBufferView): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(message);
        } else {
            console.error('WebSocket is not connected. Cannot send message.');
        }
    }
}

export default WebSocketService;