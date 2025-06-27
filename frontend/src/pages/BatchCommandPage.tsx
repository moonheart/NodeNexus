import React, { useState, useEffect, useRef } from 'react';
import type { VpsListItemResponse, CommandScript, Tag } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { executeBatchCommand, getBatchCommandWebSocket, terminateBatchCommand } from '../services/batchCommandService';
import { getCommandScripts, createCommandScript } from '../services/commandScriptService';
import SaveScriptModal from '../components/SaveScriptModal';
import Editor from '@monaco-editor/react';
import Convert from 'ansi-to-html';

const BatchCommandPage: React.FC = () => {
    const { servers } = useServerListStore();
    const [selectedVps, setSelectedVps] = useState<Set<number>>(new Set());
    const [command, setCommand] = useState('');
    const [workingDirectory, setWorkingDirectory] = useState('.');
    const [generalOutput, setGeneralOutput] = useState<string[]>([]);
    const [serverOutputs, setServerOutputs] = useState<Record<number, { name: string; logs: string[]; status: string; exitCode: number | string | null }>>({});
    const [activeView, setActiveView] = useState<'all' | 'per-server'>('all');
    const [aggregatedLogs, setAggregatedLogs] = useState<{ vpsId: number; vpsName: string; log: string }[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [showMetadata, setShowMetadata] = useState(true);
    const [isServerPanelOpen, setIsServerPanelOpen] = useState(true);
    const webSocketRef = useRef<WebSocket | null>(null);
    const ansiConverter = useRef(new Convert());
    const [currentBatchCommandId, setCurrentBatchCommandId] = useState<string | null>(null);
    const [activeServersInTask, setActiveServersInTask] = useState<Set<number>>(new Set());
    const [commandHistory, setCommandHistory] = useState<string[]>([]);
    const [showHistory, setShowHistory] = useState(false);
    const [scripts, setScripts] = useState<CommandScript[]>([]);
    const [showSaveModal, setShowSaveModal] = useState(false);
    const [scriptLanguage, setScriptLanguage] = useState('shell');
    const [editorHeight, setEditorHeight] = useState(160); // 10rem in pixels

    // Helper constants for quick select buttons
    const allOsTypes = [...new Set(servers.map(s => s.osType).filter((os): os is string => !!os))];
    const allTags: Tag[] = Array.from(new Map(servers.flatMap(s => s.tags || []).map(tag => [tag.id, tag])).values());

    // --- Selection Handlers ---

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

    const handleSelectAll = () => {
        setSelectedVps(new Set(servers.map(s => s.id)));
    };

    const handleDeselectAll = () => {
        setSelectedVps(new Set());
    };

    const handleSelectByOs = (os: string) => {
        const idsToSelect = servers.filter(s => s.osType === os).map(s => s.id);
        setSelectedVps(prev => new Set([...prev, ...idsToSelect]));
    };

    const handleSelectByTag = (tagName: string) => {
        const idsToSelect = servers.filter(s => s.tags?.some(t => t.name === tagName)).map(s => s.id);
        setSelectedVps(prev => new Set([...prev, ...idsToSelect]));
    };

    // --- End Selection Handlers ---

    useEffect(() => {
        if (!currentBatchCommandId) {
            return;
        }

        const ws = getBatchCommandWebSocket(currentBatchCommandId);
        webSocketRef.current = ws;

        ws.onopen = () => {
            setGeneralOutput(prev => [...prev, '<span class="log-content">WebSocket connection established. Waiting for output...</span>']);
        };

        ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                const { type, payload } = message;

                if (!type || !payload) {
                    setGeneralOutput(prev => [...prev, `[RAW] ${event.data}`]);
                    return;
                }

                // Get fresh data for every message to avoid stale closures
                const currentServers = useServerListStore.getState().servers;
                const server = currentServers.find(s => s.id === payload.vps_id);
                const vpsName = server ? server.name : `VPS_ID_${payload.vps_id}`;

                // This function is now defined inside `onmessage` to have access to the fresh `vpsName`.
                const updateServerOutput = (vpsId: number, log: string, statusUpdate?: Partial<{ status: string; exitCode: number | string | null }>) => {
                    setServerOutputs(prev => {
                        const newOutputs = { ...prev };
                        const existingData = newOutputs[vpsId];

                        if (!existingData) {
                            // Initialize with all available data
                            newOutputs[vpsId] = {
                                name: vpsName,
                                logs: [log],
                                status: statusUpdate?.status || 'Pending',
                                exitCode: statusUpdate?.exitCode !== undefined ? statusUpdate.exitCode : null,
                            };
                        } else {
                            // Update existing data
                            if (!existingData.logs.includes(log)) {
                                existingData.logs.push(log);
                            }
                            if (statusUpdate) {
                                existingData.status = statusUpdate.status || existingData.status;
                                existingData.exitCode = statusUpdate.exitCode !== undefined ? statusUpdate.exitCode : existingData.exitCode;
                            }
                            // Ensure name is up-to-date in case it was a placeholder
                            existingData.name = vpsName;
                        }
                        return newOutputs;
                    });
                };

                switch (type) {
                    case 'NEW_LOG_OUTPUT': {
                        const timestamp = new Date(payload.timestamp).toLocaleTimeString();
                        const rawLog = payload.log_line;
                        const formattedHtml = ansiConverter.current.toHtml(rawLog);
                        const formattedMessage = `<span class="log-meta text-gray-500">[${timestamp}] [${payload.stream_type.toUpperCase()}]: </span><span class="log-content">${formattedHtml}</span>`;
                        updateServerOutput(payload.vps_id, formattedMessage);
                        setAggregatedLogs(prev => [...prev, { vpsId: payload.vps_id, vpsName, log: formattedMessage }]);
                        break;
                    }
                    case 'CHILD_TASK_UPDATE': {
                        const timestamp = new Date().toLocaleTimeString();
                        const exitCode = payload.exit_code ?? 'N/A';
                        const formattedMessage = `<span class="log-meta text-gray-500">[${timestamp}] [STATUS]: </span><span class="log-content">Task status changed to ${payload.status}. Exit Code: ${exitCode}</span>`;
                        updateServerOutput(payload.vps_id, formattedMessage, { status: payload.status, exitCode });
                        setAggregatedLogs(prev => [...prev, { vpsId: payload.vps_id, vpsName, log: formattedMessage }]);
                        break;
                    }
                    case 'BATCH_TASK_UPDATE': {
                        const timestamp = new Date(payload.completed_at).toLocaleTimeString();
                        const formattedMessage = `<span class="log-meta text-gray-500">[${timestamp}] [SYSTEM]: </span><span class="log-content">Batch command finished with status: ${payload.overall_status}.</span>`;
                        setGeneralOutput(prev => [...prev, formattedMessage]);
                        setIsLoading(false);
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

        ws.onerror = () => {
            setError('WebSocket connection error. Check the console for details.');
            setIsLoading(false);
        };

        ws.onclose = () => {
            setGeneralOutput(prev => [...prev, '<span class="log-content">WebSocket connection closed.</span>']);
            setIsLoading(false);
            setCurrentBatchCommandId(null);
        };

        return () => {
            ws.close();
            webSocketRef.current = null;
        };

    }, [currentBatchCommandId]); // eslint-disable-line react-hooks/exhaustive-deps

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
        const updatedHistory = [newCommand, ...commandHistory].slice(0, 20);
        setCommandHistory(updatedHistory);
        localStorage.setItem('batchCommandHistory', JSON.stringify(updatedHistory));
    };

    const handleDeleteFromHistory = (indexToDelete: number) => {
        const updatedHistory = commandHistory.filter((_, index) => index !== indexToDelete);
        setCommandHistory(updatedHistory);
        localStorage.setItem('batchCommandHistory', JSON.stringify(updatedHistory));
    };

    const handleClearHistory = () => {
        setCommandHistory([]);
        localStorage.removeItem('batchCommandHistory');
        setShowHistory(false);
    };

    const handleSendCommand = async () => {
        if (selectedVps.size === 0 || command.trim() === '') return;
        if (webSocketRef.current) {
            webSocketRef.current.close();
        }
        setIsLoading(true);
        setError(null);
        setGeneralOutput(['<span class="log-content">Initiating command execution...</span>']);
        setServerOutputs({});
        setAggregatedLogs([]);
        setActiveView('all');
        setActiveServersInTask(new Set(selectedVps));

        const processedCommand = scriptLanguage === 'shell' ? command.replace(/\r\n/g, '\n') : command;

        try {
            addToHistory(command);
            const response = await executeBatchCommand(
                processedCommand,
                Array.from(selectedVps),
                workingDirectory
            );
            setGeneralOutput(prev => [...prev, `<span class="log-meta text-gray-500">[SYSTEM]: </span><span class="log-content">Batch command started with ID: ${response.batch_command_id}</span>`]);
            setCurrentBatchCommandId(response.batch_command_id);
        } catch (err: unknown) {
            console.error('Failed to execute batch command:', err);
            let errorMessage = 'An unknown error occurred.';
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
            setGeneralOutput(prev => [...prev, `<span class="log-meta text-gray-500">[SYSTEM]: </span><span class="log-content">Sending termination signal for batch command ID: ${currentBatchCommandId}...</span>`]);
            const response = await terminateBatchCommand(currentBatchCommandId);
            setGeneralOutput(prev => [...prev, `<span class="log-meta text-gray-500">[SYSTEM]: </span><span class="log-content">Termination signal acknowledged: ${response.message}</span>`]);
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
        const processedCommand = scriptLanguage === 'shell' ? command.replace(/\r\n/g, '\n') : command;
        try {
            await createCommandScript(name, description, processedCommand, workingDirectory);
            loadScripts();
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

    const handleResizeMouseDown = (e: React.MouseEvent) => {
        e.preventDefault();
        const startY = e.clientY;
        const startHeight = editorHeight;

        const doDrag = (e: MouseEvent) => {
            const newHeight = startHeight + e.clientY - startY;
            if (newHeight >= 80 && newHeight <= 800) { // min 5rem, max 50rem
                setEditorHeight(newHeight);
            }
        };

        const stopDrag = () => {
            document.removeEventListener('mousemove', doDrag);
            document.removeEventListener('mouseup', stopDrag);
        };

        document.addEventListener('mousemove', doDrag);
        document.addEventListener('mouseup', stopDrag);
    };

    return (
        <div className="container mx-auto p-4">
            <style>{`
                .hide-metadata .log-meta {
                    display: none;
                }
            `}</style>
            <h1 className="text-2xl font-bold mb-4">Batch Command Execution</h1>
            
            <div className="flex flex-col md:flex-row gap-4 md:items-start">
                {/* VPS Selector Panel */}
                <div className={`bg-white rounded-lg shadow flex flex-col flex-shrink-0 transition-all duration-300 ease-in-out ${isServerPanelOpen ? 'w-full md:w-80 p-4' : 'w-full md:w-auto p-2'}`}>
                    <div className="flex items-center">
                        <h2 className={`text-xl font-semibold mb-2 ${isServerPanelOpen ? 'block' : 'hidden'}`}>Select Servers</h2>
                        <button onClick={() => setIsServerPanelOpen(!isServerPanelOpen)} className="p-1 hover:bg-gray-200 rounded-full ml-auto">
                            <svg xmlns="http://www.w3.org/2000/svg" className={`h-6 w-6 transition-transform duration-300 ${isServerPanelOpen ? 'rotate-0' : 'rotate-180'}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                            </svg>
                        </button>
                    </div>

                    <div className={isServerPanelOpen ? 'block' : 'hidden'}>
                        {/* Quick Select Section */}
                        <div className="mb-4 p-2 border rounded-md bg-gray-50">
                        <h3 className="text-md font-semibold mb-2">Quick Select</h3>
                        <div className="flex flex-wrap gap-2 mb-2">
                            <button onClick={handleSelectAll} className="px-2 py-1 text-xs bg-blue-100 text-blue-800 hover:bg-blue-200 rounded">Select All</button>
                            <button onClick={handleDeselectAll} className="px-2 py-1 text-xs bg-gray-200 hover:bg-gray-300 rounded">Deselect All</button>
                        </div>
                        
                        {allOsTypes.length > 0 && (
                            <div className="mb-2">
                                <h4 className="text-sm font-medium text-gray-600">By OS</h4>
                                <div className="flex flex-wrap gap-2 mt-1">
                                    {allOsTypes.map(os => (
                                        <button key={os} onClick={() => handleSelectByOs(os)} className="px-2 py-1 text-xs bg-green-100 text-green-800 hover:bg-green-200 rounded">{os}</button>
                                    ))}
                                </div>
                            </div>
                        )}

                        {allTags.length > 0 && (
                            <div>
                                <h4 className="text-sm font-medium text-gray-600">By Tag</h4>
                                <div className="flex flex-wrap gap-2 mt-1">
                                    {allTags.map(tag => (
                                        <button key={tag.id} onClick={() => handleSelectByTag(tag.name)} className="px-2 py-1 text-xs text-white hover:opacity-80 rounded" style={{ backgroundColor: tag.color }}>{tag.name}</button>
                                    ))}
                                </div>
                            </div>
                        )}
                        </div>

                        <div className="space-y-2 overflow-y-auto flex-grow">
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
                </div>

                {/* Command Input and Output Panel */}
                <div className="bg-white p-4 rounded-lg shadow flex-1 min-w-0">
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
                                <select
                                    value={scriptLanguage}
                                    onChange={(e) => setScriptLanguage(e.target.value)}
                                    className="text-sm bg-gray-100 border-gray-300 rounded-md focus:ring-indigo-500 focus:border-indigo-500"
                                >
                                    <option value="shell">Shell</option>
                                    <option value="powershell">PowerShell</option>
                                </select>
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
                        <div className="relative rounded-t-md shadow-sm border border-gray-300" style={{ height: `${editorHeight}px` }}>
                            <Editor
                                language={scriptLanguage}
                                value={command}
                                onChange={(value) => setCommand(value || '')}
                                options={{
                                    minimap: { enabled: false },
                                    scrollbar: {
                                        vertical: 'auto',
                                        horizontal: 'auto'
                                    },
                                    wordWrap: 'on',
                                    lineNumbers: 'off',
                                    glyphMargin: false,
                                    folding: false,
                                    lineDecorationsWidth: 0,
                                    lineNumbersMinChars: 0,
                                    renderLineHighlight: 'none',
                                }}
                            />
                        </div>
                        <div
                            onMouseDown={handleResizeMouseDown}
                            className="w-full h-2 cursor-ns-resize bg-gray-200 hover:bg-gray-300 transition-colors rounded-b-md"
                            title="Drag to resize editor"
                        />
                        <div className="mt-2">
                        {showHistory && (
                            <div className="mt-2 p-2 border rounded-md bg-gray-50 max-h-64 overflow-y-auto">
                                {commandHistory.length > 0 ? (
                                    <>
                                        <div className="flex justify-end mb-1">
                                            <button onClick={handleClearHistory} className="text-xs text-red-500 hover:text-red-700">Clear All</button>
                                        </div>
                                        {commandHistory.map((cmd, index) => (
                                            <div key={index} className="flex items-center justify-between p-1 hover:bg-gray-200 rounded group">
                                                <div
                                                    onClick={() => {
                                                        setCommand(cmd);
                                                        setShowHistory(false);
                                                    }}
                                                    className="cursor-pointer text-sm truncate flex-grow"
                                                    title={cmd}
                                                >
                                                    {cmd}
                                                </div>
                                                <button
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        handleDeleteFromHistory(index);
                                                    }}
                                                    className="ml-2 text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
                                                    title="Delete"
                                                >
                                                    &#x2715;
                                                </button>
                                            </div>
                                        ))}
                                    </>
                                ) : (
                                    <p className="text-sm text-gray-500 text-center">No history yet.</p>
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
                        <div className="flex justify-between items-center mb-2">
                            <h3 className="text-lg font-semibold">Live Output</h3>
                            <div className="flex items-center space-x-4">
                                <div className="flex items-center">
                                    <span className="text-sm font-medium text-gray-600 mr-3">Timestamps</span>
                                    <button
                                        type="button"
                                        className={`${showMetadata ? 'bg-indigo-600' : 'bg-gray-200'} relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2`}
                                        role="switch"
                                        aria-checked={showMetadata}
                                        onClick={() => setShowMetadata(!showMetadata)}
                                    >
                                        <span
                                            aria-hidden="true"
                                            className={`${showMetadata ? 'translate-x-5' : 'translate-x-0'} pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out`}
                                        ></span>
                                    </button>
                                </div>
                                <div className="flex items-center space-x-2">
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
                            </div>
                        </div>

                        <div className={`bg-gray-900 text-white p-4 rounded-md font-mono text-sm h-96 overflow-y-auto ${!showMetadata ? 'hide-metadata' : ''}`}>
                            {activeView === 'all' && (
                                <>
                                    {generalOutput.map((line, index) => (
                                        <div key={`general-${index}`} style={{ whiteSpace: 'pre-wrap' }} dangerouslySetInnerHTML={{ __html: line }} />
                                    ))}
                                    {aggregatedLogs.map((item, index) => (
                                       <div key={`agg-${index}`} style={{ whiteSpace: 'pre-wrap' }}>
                                           <span className="text-cyan-400 mr-2">[{item.vpsName}]</span>
                                           <span dangerouslySetInnerHTML={{ __html: item.log }} />
                                       </div>
                                    ))}
                                    {generalOutput.length === 0 && aggregatedLogs.length === 0 && (
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
                                                        <div key={index} style={{ whiteSpace: 'pre-wrap' }} dangerouslySetInnerHTML={{ __html: log }} />
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