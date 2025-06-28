
// This type might need to be expanded based on the actual API response
export interface BatchCommandResponse {
    batch_command_id: string;
    // other fields from the response...
}

/**
 * Creates and returns a WebSocket connection for a new batch command execution.
 * The caller is responsible for handling onopen, onmessage, etc.,
 * and for sending the command payload once the connection is open.
 * @returns A WebSocket instance.
 */
export const connectForBatchCommand = (): WebSocket => {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    // Connect to the new endpoint that handles WebSocket upgrades.
    const wsUrl = `${wsProtocol}//${window.location.host}/api/batch_commands`;
    return new WebSocket(wsUrl);
};