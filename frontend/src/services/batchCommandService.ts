import apiClient from './apiClient';

// This type might need to be expanded based on the actual API response
export interface BatchCommandResponse {
    batch_command_id: string;
    // other fields from the response...
}

/**
 * Initiates a batch command execution.
 * @param command The command string to execute.
 * @param vps_db_ids An array of database IDs for the target VPS.
 * @param working_directory The directory where the command should be executed.
 */
export const executeBatchCommand = async (
    command: string,
    vps_db_ids: number[],
    working_directory: string
): Promise<BatchCommandResponse> => {
    const payload = {
        command_content: command,
        target_vps_ids: vps_db_ids,
        working_directory,
    };
    const response = await apiClient.post<BatchCommandResponse>('/batch_commands', payload);
    return response.data;
};

/**
 * Creates and returns a WebSocket connection for receiving batch command output.
 * @param batchCommandId The ID of the batch command task.
 * @returns A WebSocket instance.
 */
export const getBatchCommandWebSocket = (batchCommandId: string): WebSocket => {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProtocol}//${window.location.host}/ws/batch-command/${batchCommandId}`;
    return new WebSocket(wsUrl);
};
/**
 * Sends a request to terminate a running batch command.
 * @param batchCommandId The ID of the batch command to terminate.
 */
export interface TerminateBatchCommandResponse {
    message: string;
}

export const terminateBatchCommand = async (batchCommandId: string): Promise<TerminateBatchCommandResponse> => {
    // The backend might return a confirmation message.
    const response = await apiClient.post<TerminateBatchCommandResponse>(`/batch_commands/${batchCommandId}/terminate`);
    return response.data;
};