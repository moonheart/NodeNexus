import React, { useState, useEffect, useRef } from 'react';
import type { VpsListItemResponse } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { executeBatchCommand, getBatchCommandWebSocket, terminateBatchCommand } from '../services/batchCommandService';

const BatchCommandPage: React.FC = () => {
    const { servers } = useServerListStore();
    const [selectedVps, setSelectedVps] = useState<Set<number>>(new Set());
    const [command, setCommand] = useState('');
    const [workingDirectory, setWorkingDirectory] = useState('.');
    const [output, setOutput] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const webSocketRef = useRef<WebSocket | null>(null);
    const [currentBatchCommandId, setCurrentBatchCommandId] = useState<string | null>(null);

    // Cleanup WebSocket on component unmount
    useEffect(() => {
        return () => {
            if (webSocketRef.current) {
                webSocketRef.current.close();
            }
        };
    }, []);

    const handleVpsSelection = (vpsId: number) => {
        setSelectedVps(prevSelected => {
            const newSelected = new Set(prevSelected);
            if (newSelected.has(vpsId)) {
                newSelected.delete(vpsId);
            } else {
                newSelected.add(vpsId);
            }
            return newSelected;
        });
    };

    const handleSendCommand = async () => {
        if (selectedVps.size === 0 || command.trim() === '') return;

        setIsLoading(true);
        setError(null);
        setOutput(['Initiating command execution...']);

        try {
            const response = await executeBatchCommand(
                command,
                Array.from(selectedVps),
                workingDirectory
            );
            
            const { batch_command_id } = response;
            setCurrentBatchCommandId(batch_command_id); // Save the batch command ID
            setOutput(prev => [...prev, `Batch command started with ID: ${batch_command_id}`]);

            // Establish WebSocket connection
            const ws = getBatchCommandWebSocket(batch_command_id);
            webSocketRef.current = ws;

            ws.onopen = () => {
                setOutput(prev => [...prev, 'WebSocket connection established. Waiting for output...']);
            };

            ws.onmessage = (event) => {
                try {
                    const message = JSON.parse(event.data);
                    const { type, payload } = message;

                    if (!type || !payload) {
                        setOutput(prev => [...prev, `[RAW] ${event.data}`]);
                        return;
                    }

                    const server = servers.find(s => s.id === payload.vps_id);
                    const vpsName = server ? server.name : `VPS_ID_${payload.vps_id}`;
                    let formattedMessage = '';

                    switch (type) {
                        case 'NEW_LOG_OUTPUT': {
                            const timestamp = new Date(payload.timestamp).toLocaleTimeString();
                            formattedMessage = `[${timestamp}] [${vpsName}] [${payload.stream_type.toUpperCase()}]: ${payload.log_line.trim()}`;
                            break;
                        }
                        case 'CHILD_TASK_UPDATE': {
                            const timestamp = new Date().toLocaleTimeString();
                            formattedMessage = `[${timestamp}] [${vpsName}] [STATUS]: Task status changed to ${payload.status}. Exit Code: ${payload.exit_code ?? 'N/A'}`;
                            break;
                        }
                        case 'BATCH_TASK_UPDATE': {
                            const timestamp = new Date(payload.completed_at).toLocaleTimeString();
                            formattedMessage = `[${timestamp}] [SYSTEM] [STATUS]: Batch command finished with status: ${payload.overall_status}.`;
                            setIsLoading(false); // Reset the button state
                            if (webSocketRef.current) {
                                webSocketRef.current.close();
                            }
                            setCurrentBatchCommandId(null); // Clear the command ID
                            break;
                        }
                        default:
                            formattedMessage = `[UNKNOWN] ${event.data}`;
                            break;
                    }
                    setOutput(prev => [...prev, formattedMessage]);

                } catch (e) {
                    console.error('Failed to parse or process WebSocket message:', e);
                    setOutput(prev => [...prev, `[RAW] ${event.data}`]);
                }
            };

            ws.onerror = (event) => {
                console.error('WebSocket error:', event);
                setError('WebSocket connection error. Check the console for details.');
                setIsLoading(false);
            };

            ws.onclose = () => {
                setOutput(prev => [...prev, 'WebSocket connection closed.']);
                setIsLoading(false);
                if (webSocketRef.current?.readyState === WebSocket.OPEN) {
                    webSocketRef.current.close();
                }
                setCurrentBatchCommandId(null);
            };

        } catch (err: unknown) {
            console.error('Failed to execute batch command:', err);
            let errorMessage = 'An unknown error occurred.';
            
            // Type-safe error message extraction
            if (typeof err === 'object' && err !== null) {
                const potentialError = err as { response?: { data?: { message?: string } }, message?: string };
                if (potentialError.response?.data?.message) {
                    errorMessage = potentialError.response.data.message;
                } else if (potentialError.message) {
                    errorMessage = potentialError.message;
                }
            }

            setError(`Failed to start command: ${errorMessage}`);
            setIsLoading(false);
        }
    };

    const handleTerminateCommand = async () => {
        if (!currentBatchCommandId) {
            setError("No active command to terminate.");
            return;
        }
        try {
            setOutput(prev => [...prev, `[SYSTEM] Sending termination signal for batch command ID: ${currentBatchCommandId}...`]);
            const response = await terminateBatchCommand(currentBatchCommandId);
            setOutput(prev => [...prev, `[SYSTEM] Termination signal acknowledged: ${response.message}`]);
            // The WebSocket 'onclose' or a 'BATCH_TASK_UPDATE' message should handle the final state change.
        } catch (err: unknown) {
            console.error('Failed to terminate batch command:', err);
            let errorMessage = 'An unknown error occurred during termination.';
            if (typeof err === 'object' && err !== null) {
                const potentialError = err as { response?: { data?: { message?: string } }, message?: string };
                if (potentialError.response?.data?.message) {
                    errorMessage = potentialError.response.data.message;
                } else if (potentialError.message) {
                    errorMessage = potentialError.message;
                }
            }
            setError(`Failed to terminate command: ${errorMessage}`);
        }
    };

    return (
        <div className="container mx-auto p-4">
            <h1 className="text-2xl font-bold mb-4">Batch Command Execution</h1>
            
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {/* VPS Selector Panel */}
                <div className="md:col-span-1 bg-white p-4 rounded-lg shadow">
                    <h2 className="text-xl font-semibold mb-2">Select Servers</h2>
                    <div className="space-y-2 max-h-96 overflow-y-auto">
                        {servers.map((vps: VpsListItemResponse) => (
                            <div key={vps.id} className="flex items-center">
                                <input
                                    type="checkbox"
                                    id={`vps-${vps.id}`}
                                    checked={selectedVps.has(vps.id)}
                                    onChange={() => handleVpsSelection(vps.id)}
                                    className="h-4 w-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
                                />
                                <label htmlFor={`vps-${vps.id}`} className="ml-2 block text-sm text-gray-900">
                                    {vps.name} ({vps.ipAddress})
                                </label>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Command Input and Output Panel */}
                <div className="md:col-span-2 bg-white p-4 rounded-lg shadow">
                    <div className="mb-4">
                        <label htmlFor="working-directory-input" className="block text-sm font-medium text-gray-700 mb-1">
                            Working Directory
                        </label>
                        <input
                            id="working-directory-input"
                            type="text"
                            className="w-full p-2 border border-gray-300 rounded-md shadow-sm focus:ring-indigo-500 focus:border-indigo-500"
                            value={workingDirectory}
                            onChange={(e) => setWorkingDirectory(e.target.value)}
                            placeholder="e.g., /root or C:\Users\Admin"
                        />
                    </div>

                    <div className="mb-4">
                        <label htmlFor="command-input" className="block text-sm font-medium text-gray-700 mb-1">
                            Command
                        </label>
                        <textarea
                            id="command-input"
                            rows={4}
                            className="w-full p-2 border border-gray-300 rounded-md shadow-sm focus:ring-indigo-500 focus:border-indigo-500"
                            value={command}
                            onChange={(e) => setCommand(e.target.value)}
                            placeholder="Enter command to execute on selected servers..."
                        />
                    </div>
                    
                    <div className="flex space-x-2">
                        <button
                            onClick={handleSendCommand}
                            disabled={selectedVps.size === 0 || command.trim() === '' || isLoading}
                            className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded disabled:bg-gray-400"
                        >
                            {isLoading ? 'Executing...' : 'Run Command'}
                        </button>
                        {isLoading && currentBatchCommandId && (
                             <button
                                onClick={handleTerminateCommand}
                                className="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                            >
                                Terminate
                            </button>
                        )}
                    </div>

                    {error && (
                        <div className="mt-4 p-2 bg-red-100 text-red-700 border border-red-400 rounded">
                            {error}
                        </div>
                    )}

                    <div className="mt-4">
                        <h3 className="text-lg font-semibold mb-2">Live Output</h3>
                        <div className="bg-gray-900 text-white p-4 rounded-md font-mono text-sm h-96 overflow-y-auto">
                            {output.length > 0 ? (
                                output.map((line, index) => (
                                    <div key={index} style={{ whiteSpace: 'pre-wrap' }}>{line}</div>
                                ))
                            ) : (
                                <p>Command output will appear here...</p>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default BatchCommandPage;