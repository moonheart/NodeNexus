import React, { useState, useEffect, useRef } from 'react';
import type { VpsListItemResponse, CommandScript } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { executeBatchCommand, getBatchCommandWebSocket, terminateBatchCommand } from '../services/batchCommandService';
import { getCommandScripts, createCommandScript } from '../services/commandScriptService';
import SaveScriptModal from '../components/SaveScriptModal';

const BatchCommandPage: React.FC = () => {
    const { servers } = useServerListStore();
    const [selectedVps, setSelectedVps] = useState<Set<number>>(new Set());
    const [command, setCommand] = useState('');
    const [workingDirectory, setWorkingDirectory] = useState('.');
    const [generalOutput, setGeneralOutput] = useState<string[]>([]);
    const [serverOutputs, setServerOutputs] = useState<Record<number, { name: string; logs: string[]; status: string; exitCode: number | string | null }>>({});
    const [activeView, setActiveView] = useState<'all' | 'per-server'>('all');
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const webSocketRef = useRef<WebSocket | null>(null);
    const [currentBatchCommandId, setCurrentBatchCommandId] = useState<string | null>(null);
    const [activeServersInTask, setActiveServersInTask] = useState<Set<number>>(new Set());
    const [commandHistory, setCommandHistory] = useState<string[]>([]);
    const [showHistory, setShowHistory] = useState(false);
    const [scripts, setScripts] = useState<CommandScript[]>([]);
    const [showSaveModal, setShowSaveModal] = useState(false);

    // Cleanup WebSocket on component unmount
    useEffect(() => {
        return () => {
            if (webSocketRef.current) {
                webSocketRef.current.close();
            }
        };
    }, []);

    useEffect(() => {
        const storedHistory = localStorage.getItem('batchCommandHistory');
        if (storedHistory) {
            setCommandHistory(JSON.parse(storedHistory));
        }
        loadScripts();
    }, []);

    const loadScripts = async () => {
        try {
            const fetchedScripts = await getCommandScripts();
            setScripts(fetchedScripts);
        } catch (error) {
            console.error("Failed to load scripts:", error);
            setError("Failed to load saved scripts.");
        }
    };

    const addToHistory = (newCommand: string) => {
        if (!newCommand || commandHistory.includes(newCommand)) return;

        const updatedHistory = [newCommand, ...commandHistory].slice(0, 20); // Keep last 20
        setCommandHistory(updatedHistory);
        localStorage.setItem('batchCommandHistory', JSON.stringify(updatedHistory));
    };

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
        setGeneralOutput(['Initiating command execution...']);
        setServerOutputs({});
        setActiveView('all');
        setActiveServersInTask(new Set(selectedVps));

        try {
            addToHistory(command); // Add to history on execution
            const response = await executeBatchCommand(
                command,
                Array.from(selectedVps),
                workingDirectory
            );

            const { batch_command_id } = response;
            setCurrentBatchCommandId(batch_command_id); // Save the batch command ID
            setGeneralOutput(prev => [...prev, `Batch command started with ID: ${batch_command_id}`]);

            // Establish WebSocket connection
            const ws = getBatchCommandWebSocket(batch_command_id);
            webSocketRef.current = ws;

            ws.onopen = () => {
                setGeneralOutput(prev => [...prev, 'WebSocket connection established. Waiting for output...']);
            };

            ws.onmessage = (event) => {
                try {
                    const message = JSON.parse(event.data);
                    const { type, payload } = message;

                    if (!type || !payload) {
                        setGeneralOutput(prev => [...prev, `[RAW] ${event.data}`]);
                        return;
                    }

                    const server = servers.find(s => s.id === payload.vps_id);
                    const vpsName = server ? server.name : `VPS_ID_${payload.vps_id}`;

                    const updateServerOutput = (vpsId: number, log: string, statusUpdate?: Partial<{ status: string; exitCode: number | string | null }>) => {
                        setServerOutputs(prev => {
                            const newOutputs = { ...prev };
                            if (!newOutputs[vpsId]) {
                                newOutputs[vpsId] = {
                                    name: vpsName,
                                    logs: [],
                                    status: 'Pending',
                                    exitCode: null,
                                };
                            }
                            newOutputs[vpsId].logs.push(log);
                            if (statusUpdate) {
                                newOutputs[vpsId] = { ...newOutputs[vpsId], ...statusUpdate };
                            }
                            return newOutputs;
                        });
                    };

                    switch (type) {
                        case 'NEW_LOG_OUTPUT': {
                            const timestamp = new Date(payload.timestamp).toLocaleTimeString();
                            const formattedMessage = `[${timestamp}] [${payload.stream_type.toUpperCase()}]: ${payload.log_line.trim()}`;
                            updateServerOutput(payload.vps_id, formattedMessage);
                            break;
                        }
                        case 'CHILD_TASK_UPDATE': {
                            const timestamp = new Date().toLocaleTimeString();
                            const exitCode = payload.exit_code ?? 'N/A';
                            const formattedMessage = `[${timestamp}] [STATUS]: Task status changed to ${payload.status}. Exit Code: ${exitCode}`;
                            updateServerOutput(payload.vps_id, formattedMessage, { status: payload.status, exitCode });
                            break;
                        }
                        case 'BATCH_TASK_UPDATE': {
                            const timestamp = new Date(payload.completed_at).toLocaleTimeString();
                            const formattedMessage = `[${timestamp}] [SYSTEM] [STATUS]: Batch command finished with status: ${payload.overall_status}.`;
                            setGeneralOutput(prev => [...prev, formattedMessage]);
                            setIsLoading(false);
                            if (webSocketRef.current) {
                                webSocketRef.current.close();
                            }
                            setCurrentBatchCommandId(null);
                            break;
                        }
                        default:
                            setGeneralOutput(prev => [...prev, `[UNKNOWN] ${event.data}`]);
                            break;
                    }

                } catch (e) {
                    console.error('Failed to parse or process WebSocket message:', e);
                    setGeneralOutput(prev => [...prev, `[RAW] ${event.data}`]);
                }
            };

            ws.onerror = (event) => {
                console.error('WebSocket error:', event);
                setError('WebSocket connection error. Check the console for details.');
                setIsLoading(false);
            };

            ws.onclose = () => {
                setGeneralOutput(prev => [...prev, 'WebSocket connection closed.']);
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
            setGeneralOutput(prev => [...prev, `[SYSTEM] Sending termination signal for batch command ID: ${currentBatchCommandId}...`]);
            const response = await terminateBatchCommand(currentBatchCommandId);
            setGeneralOutput(prev => [...prev, `[SYSTEM] Termination signal acknowledged: ${response.message}`]);
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

    const handleSaveScript = async (name: string, description: string) => {
        try {
            await createCommandScript(name, description, command, workingDirectory);
            loadScripts(); // Refresh script list
        } catch (err) {
            console.error("Failed to save script:", err);
            let errorMessage = 'An unknown error occurred while saving the script.';
            if (typeof err === 'object' && err !== null) {
                const potentialError = err as { response?: { data?: { error?: string } } };
                if (potentialError.response?.data?.error) {
                    errorMessage = potentialError.response.data.error;
                }
            }
            setError(errorMessage);
        }
    };

    const handleSelectScript = (script: CommandScript) => {
        setCommand(script.script_content);
        setWorkingDirectory(script.working_directory);
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
                        <div className="flex justify-between items-center mb-1">
                            <label htmlFor="command-input" className="block text-sm font-medium text-gray-700">
                                Command
                            </label>
                            <div className="flex items-center space-x-4">
                                <div className="relative">
                                    <select
                                        onChange={(e) => {
                                            const scriptId = parseInt(e.target.value, 10);
                                            const script = scripts.find(s => s.id === scriptId);
                                            if (script) handleSelectScript(script);
                                        }}
                                        className="text-sm text-indigo-600 hover:text-indigo-800 bg-transparent border-none focus:ring-0"
                                        defaultValue=""
                                    >
                                        <option value="" disabled>Load Script...</option>
                                        {scripts.map(script => (
                                            <option key={script.id} value={script.id}>{script.name}</option>
                                        ))}
                                    </select>
                                </div>
                                <button
                                    onClick={() => setShowHistory(!showHistory)}
                                    className="text-sm text-indigo-600 hover:text-indigo-800"
                                >
                                    {showHistory ? 'Hide' : 'Show'} History
                                </button>
                            </div>
                        </div>
                        <textarea
                            id="command-input"
                            rows={4}
                            className="w-full p-2 border border-gray-300 rounded-md shadow-sm focus:ring-indigo-500 focus:border-indigo-500"
                            value={command}
                            onChange={(e) => setCommand(e.target.value)}
                            placeholder="Enter command to execute on selected servers..."
                        />
                        {showHistory && (
                            <div className="mt-2 p-2 border rounded-md bg-gray-50 max-h-32 overflow-y-auto">
                                {commandHistory.length > 0 ? (
                                    commandHistory.map((cmd, index) => (
                                        <div
                                            key={index}
                                            onClick={() => {
                                                setCommand(cmd);
                                                setShowHistory(false);
                                            }}
                                            className="cursor-pointer p-1 hover:bg-gray-200 rounded text-sm"
                                        >
                                            {cmd}
                                        </div>
                                    ))
                                ) : (
                                    <p className="text-sm text-gray-500">No history yet.</p>
                                )}
                            </div>
                        )}
                    </div>

                    <div className="flex space-x-2">
                        <button
                            onClick={handleSendCommand}
                            disabled={selectedVps.size === 0 || command.trim() === '' || isLoading}
                            className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded disabled:bg-gray-400"
                        >
                            {isLoading ? 'Executing...' : 'Run Command'}
                        </button>
                        <button
                            onClick={() => setShowSaveModal(true)}
                            disabled={command.trim() === ''}
                            className="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded disabled:bg-gray-400"
                        >
                            Save as Script
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
                        {/* View Toggles */}
                        <div className="flex items-center space-x-2 mb-2">
                            <button
                                onClick={() => setActiveView('all')}
                                className={`px-3 py-1 text-sm rounded-md ${activeView === 'all' ? 'bg-indigo-600 text-white' : 'bg-gray-200 text-gray-700'}`}
                            >
                                Aggregated View
                            </button>
                            <button
                                onClick={() => setActiveView('per-server')}
                                className={`px-3 py-1 text-sm rounded-md ${activeView === 'per-server' ? 'bg-indigo-600 text-white' : 'bg-gray-200 text-gray-700'}`}
                            >
                                Per-Server View
                            </button>
                        </div>

                        <div className="bg-gray-900 text-white p-4 rounded-md font-mono text-sm h-96 overflow-y-auto">
                            {activeView === 'all' && (
                                <>
                                    {generalOutput.map((line, index) => (
                                        <div key={`general-${index}`} style={{ whiteSpace: 'pre-wrap' }}>{line}</div>
                                    ))}
                                    {Object.entries(serverOutputs).map(([vpsId, data]) =>
                                        data.logs.map((log, logIndex) => (
                                            <div key={`${vpsId}-${logIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                                                <span className="text-cyan-400 mr-2">[{data.name}]</span>{log}
                                            </div>
                                        ))
                                    )}
                                    {generalOutput.length === 0 && Object.keys(serverOutputs).length === 0 && (
                                        <p>Command output will appear here...</p>
                                    )}
                                </>
                            )}

                            {activeView === 'per-server' && (
                                <>
                                    {Array.from(activeServersInTask).map(vpsId => {
                                        const data = serverOutputs[vpsId];
                                        const server = servers.find(s => s.id === vpsId);
                                        const vpsName = server ? server.name : `VPS_ID_${vpsId}`;

                                        if (!data) {
                                            return (
                                                <details key={vpsId} className="mb-2">
                                                    <summary className="cursor-pointer font-semibold text-gray-400">
                                                        {vpsName} - <span className="text-yellow-400">Pending...</span>
                                                    </summary>
                                                </details>
                                            );
                                        }

                                        const statusColor = data.status.toLowerCase().includes('success') || (data.exitCode === 0)
                                            ? 'text-green-400'
                                            : data.status.toLowerCase().includes('fail') || (typeof data.exitCode === 'number' && data.exitCode > 0)
                                            ? 'text-red-400'
                                            : 'text-yellow-400';

                                        return (
                                            <details key={vpsId} className="mb-2" open>
                                                <summary className="cursor-pointer font-semibold">
                                                    {data.name} - <span className={statusColor}>{data.status} (Exit: {data.exitCode ?? 'N/A'})</span>
                                                </summary>
                                                <div className="pl-4 mt-2 border-l-2 border-gray-700">
                                                    {data.logs.map((log, index) => (
                                                        <div key={index} style={{ whiteSpace: 'pre-wrap' }}>{log}</div>
                                                    ))}
                                                </div>
                                            </details>
                                        );
                                    })}
                                    {activeServersInTask.size === 0 && <p>No servers selected for the command.</p>}
                                </>
                            )}
                        </div>
                    </div>
                </div>
            </div>
            <SaveScriptModal
                isOpen={showSaveModal}
                onClose={() => setShowSaveModal(false)}
                onSave={handleSaveScript}
                initialCommand={command}
            />
        </div>
    );
};

export default BatchCommandPage;