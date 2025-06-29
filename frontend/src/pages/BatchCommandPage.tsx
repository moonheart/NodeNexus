import React, { useState, useEffect, useRef } from 'react';
import type { VpsListItemResponse, CommandScript, Tag } from '../types';
import { useServerListStore } from '../store/serverListStore';
import { connectForBatchCommand } from '../services/batchCommandService';
import { getCommandScripts, createCommandScript } from '../services/commandScriptService';
import SaveScriptModal from '../components/SaveScriptModal';
import Editor from '@monaco-editor/react';
import { useTheme } from "@/components/ThemeProvider";
import Convert from 'ansi-to-html';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { ChevronLeft, History, X } from 'lucide-react';

const BatchCommandPage: React.FC = () => {
    const { themeType } = useTheme();
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

    const allOsTypes = [...new Set(servers.map(s => s.osType).filter((os): os is string => !!os))];
    const allTags: Tag[] = Array.from(new Map(servers.flatMap(s => s.tags || []).map(tag => [tag.id, tag])).values());

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

    const handleSelectAll = () => setSelectedVps(new Set(servers.map(s => s.id)));
    const handleDeselectAll = () => setSelectedVps(new Set());
    const handleSelectByOs = (os: string) => {
        const idsToSelect = servers.filter(s => s.osType === os).map(s => s.id);
        setSelectedVps(prev => new Set([...prev, ...idsToSelect]));
    };
    const handleSelectByTag = (tagName: string) => {
        const idsToSelect = servers.filter(s => s.tags?.some(t => t.name === tagName)).map(s => s.id);
        setSelectedVps(prev => new Set([...prev, ...idsToSelect]));
    };

    useEffect(() => {
        const storedHistory = localStorage.getItem('batchCommandHistory');
        if (storedHistory) setCommandHistory(JSON.parse(storedHistory));
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

    const handleSendCommand = () => {
        if (selectedVps.size === 0 || command.trim() === '') return;
        if (webSocketRef.current) webSocketRef.current.close();

        setIsLoading(true);
        setError(null);
        setGeneralOutput(['<span class="log-content">Connecting to server...</span>']);
        setServerOutputs({});
        setAggregatedLogs([]);
        setActiveView('all');
        setActiveServersInTask(new Set(selectedVps));
        setCurrentBatchCommandId(null);

        addToHistory(command);
        const processedCommand = scriptLanguage === 'shell' ? command.replace(/\r\n/g, '\n') : command;

        const ws = connectForBatchCommand();
        webSocketRef.current = ws;

        ws.onopen = () => {
            setGeneralOutput(prev => [...prev, '<span class="log-content">Connection established. Sending command...</span>']);
            ws.send(JSON.stringify({
                command_content: processedCommand,
                target_vps_ids: Array.from(selectedVps),
                working_directory: workingDirectory,
            }));
        };

        ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                const { type, payload } = message;
                if (!type || !payload) {
                    setGeneralOutput(prev => [...prev, `[RAW] ${event.data}`]);
                    return;
                }

                const vpsName = useServerListStore.getState().servers.find(s => s.id === payload.vps_id)?.name || `VPS_ID_${payload.vps_id}`;

                const updateServerOutput = (vpsId: number, log: string, statusUpdate?: Partial<{ status: string; exitCode: number | string | null }>) => {
                    setServerOutputs(prev => ({
                        ...prev,
                        [vpsId]: {
                            name: vpsName,
                            logs: [...(prev[vpsId]?.logs || []), log],
                            status: statusUpdate?.status || prev[vpsId]?.status || 'Pending',
                            exitCode: statusUpdate?.exitCode !== undefined ? statusUpdate.exitCode : prev[vpsId]?.exitCode,
                        },
                    }));
                };

                switch (type) {
                    case 'BATCH_TASK_CREATED':
                        setGeneralOutput(prev => [...prev, `<span class="log-meta text-gray-500">[SYSTEM]: </span><span class="log-content">Batch command started with ID: ${payload.batch_command_id}</span>`]);
                        setCurrentBatchCommandId(payload.batch_command_id);
                        break;
                    case 'NEW_LOG_OUTPUT': {
                        const formattedHtml = ansiConverter.current.toHtml(payload.log_line);
                        const formattedMessage = `<span class="log-meta text-gray-500">[${new Date(payload.timestamp).toLocaleTimeString()}] [${payload.stream_type.toUpperCase()}]: </span><span class="log-content">${formattedHtml}</span>`;
                        updateServerOutput(payload.vps_id, formattedMessage);
                        setAggregatedLogs(prev => [...prev, { vpsId: payload.vps_id, vpsName, log: formattedMessage }]);
                        break;
                    }
                    case 'CHILD_TASK_UPDATE': {
                        const formattedMessage = `<span class="log-meta text-gray-500">[${new Date().toLocaleTimeString()}] [STATUS]: </span><span class="log-content">Task status changed to ${payload.status}. Exit Code: ${payload.exit_code ?? 'N/A'}</span>`;
                        updateServerOutput(payload.vps_id, formattedMessage, { status: payload.status, exitCode: payload.exit_code });
                        setAggregatedLogs(prev => [...prev, { vpsId: payload.vps_id, vpsName, log: formattedMessage }]);
                        break;
                    }
                    case 'BATCH_TASK_UPDATE': {
                        const formattedMessage = `<span class="log-meta text-gray-500">[${new Date(payload.completed_at).toLocaleTimeString()}] [SYSTEM]: </span><span class="log-content">Batch command finished with status: ${payload.overall_status}.</span>`;
                        setGeneralOutput(prev => [...prev, formattedMessage]);
                        if (["CompletedSuccessfully", "CompletedWithErrors", "Terminated", "FailedToDispatch"].includes(payload.overall_status)) {
                            setIsLoading(false);
                            if (webSocketRef.current) webSocketRef.current.close();
                        }
                        break;
                    }
                    default:
                        setGeneralOutput(prev => [...prev, `[UNKNOWN] ${event.data}`]);
                }
            } catch (e) {
                console.error('Failed to parse or process WebSocket message:', e);
                setGeneralOutput(prev => [...prev, `[RAW] ${event.data}`]);
            }
        };

        ws.onerror = (event) => {
            console.error('WebSocket connection error:', event);
            setError('WebSocket connection error. Could not connect to the server.');
            setIsLoading(false);
        };

        ws.onclose = (event) => {
            setGeneralOutput(prev => [...prev, `<span class="log-content">WebSocket connection closed. Code: ${event.code}</span>`]);
            setIsLoading(false);
        };
    };

    const handleTerminateCommand = () => {
        if (!currentBatchCommandId) {
            setError("No active command to terminate.");
            return;
        }
        if (!webSocketRef.current || webSocketRef.current.readyState !== WebSocket.OPEN) {
            setError("WebSocket is not connected. Cannot send termination signal.");
            return;
        }
        setGeneralOutput(prev => [...prev, `<span class="log-meta text-gray-500">[SYSTEM]: </span><span class="log-content">Sending termination signal for batch command ID: ${currentBatchCommandId}...</span>`]);
        webSocketRef.current.send(JSON.stringify({ type: "TERMINATE_TASK" }));
    };

    const handleSaveScript = async (name: string, description: string) => {
        const processedCommand = scriptLanguage === 'shell' ? command.replace(/\r\n/g, '\n') : command;
        try {
            await createCommandScript(name, description, processedCommand, workingDirectory);
            loadScripts();
        } catch (err) {
            console.error("Failed to save script:", err);
            type AxiosError = {
                response?: {
                    data?: {
                        error?: string;
                    };
                };
            };
            const axiosError = err as AxiosError;
            const errorMsg = axiosError.response?.data?.error || 'An unknown error occurred while saving the script.';
            setError(errorMsg);
        }
    };

    const handleSelectScript = (scriptId: string) => {
        const script = scripts.find(s => s.id === parseInt(scriptId, 10));
        if (script) {
            setCommand(script.script_content);
            setWorkingDirectory(script.working_directory);
        }
    };

    const handleResizeMouseDown = (e: React.MouseEvent) => {
        e.preventDefault();
        const startY = e.clientY;
        const startHeight = editorHeight;
        const doDrag = (e: MouseEvent) => {
            const newHeight = startHeight + e.clientY - startY;
            if (newHeight >= 80 && newHeight <= 800) setEditorHeight(newHeight);
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
            <style>{`.hide-metadata .log-meta { display: none; }`}</style>
            <h1 className="text-2xl font-bold mb-4">Batch Command Execution</h1>
            
            <div className="flex flex-col md:flex-row gap-4 md:items-start">
                <Card className={`transition-all duration-300 ease-in-out ${isServerPanelOpen ? 'w-full md:w-80' : 'w-auto'}`}>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className={`text-lg font-semibold ${isServerPanelOpen ? 'block' : 'hidden'}`}>Select Servers</CardTitle>
                        <Button variant="ghost" size="icon" onClick={() => setIsServerPanelOpen(!isServerPanelOpen)} className="rounded-full">
                            <ChevronLeft className={`h-5 w-5 transition-transform duration-300 ${isServerPanelOpen ? 'rotate-0' : 'rotate-180'}`} />
                        </Button>
                    </CardHeader>
                    <CardContent className={isServerPanelOpen ? 'block' : 'hidden'}>
                        <Card className="mb-4 bg-muted/50 y-0">
                            <CardHeader className="pb-2 pt-4">
                                <CardTitle className="text-base">Quick Select</CardTitle>
                            </CardHeader>
                            <CardContent>
                                <div className="flex flex-wrap gap-2 mb-2">
                                    <Button onClick={handleSelectAll} size="sm" variant="secondary">Select All</Button>
                                    <Button onClick={handleDeselectAll} size="sm" variant="secondary">Deselect All</Button>
                                </div>
                                {allOsTypes.length > 0 && (
                                    <div className="mb-2">
                                        <h4 className="text-sm font-medium text-muted-foreground mb-1">By OS</h4>
                                        <div className="flex flex-wrap gap-2">
                                            {allOsTypes.map(os => <Badge key={os} onClick={() => handleSelectByOs(os)} className="cursor-pointer">{os}</Badge>)}
                                        </div>
                                    </div>
                                )}
                                {allTags.length > 0 && (
                                    <div>
                                        <h4 className="text-sm font-medium text-muted-foreground mb-1">By Tag</h4>
                                        <div className="flex flex-wrap gap-2">
                                            {allTags.map(tag => <Badge key={tag.id} onClick={() => handleSelectByTag(tag.name)} className="cursor-pointer" style={{ backgroundColor: tag.color }}>{tag.name}</Badge>)}
                                        </div>
                                    </div>
                                )}
                            </CardContent>
                        </Card>
                        <div className="space-y-2 overflow-y-auto flex-grow max-h-96">
                            {servers.map((vps: VpsListItemResponse) => (
                                <div key={vps.id} className="flex items-center space-x-2">
                                    <Checkbox id={`vps-${vps.id}`} checked={selectedVps.has(vps.id)} onCheckedChange={() => handleVpsSelection(vps.id)} />
                                    <Label htmlFor={`vps-${vps.id}`} className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                        {vps.name} <span className="text-muted-foreground">({vps.ipAddress})</span>
                                    </Label>
                                </div>
                            ))}
                        </div>
                    </CardContent>
                </Card>

                <Card className="flex-1 min-w-0">
                    <CardContent className="p-4">
                        <div className="grid gap-4">
                            <div>
                                <Label htmlFor="working-directory-input">Working Directory</Label>
                                <Input id="working-directory-input" value={workingDirectory} onChange={(e) => setWorkingDirectory(e.target.value)} placeholder="e.g., /root or C:\Users\Admin" />
                            </div>
                            <div className="min-w-0">
                                <div className="flex justify-between items-center mb-1">
                                    <Label htmlFor="command-input">Command</Label>
                                    <div className="flex items-center gap-2">
                                        <Select value={scriptLanguage} onValueChange={setScriptLanguage}>
                                            <SelectTrigger className="w-[120px] h-8 text-xs">
                                                <SelectValue placeholder="Language" />
                                            </SelectTrigger>
                                            <SelectContent>
                                                <SelectItem value="shell">Shell</SelectItem>
                                                <SelectItem value="powershell">PowerShell</SelectItem>
                                            </SelectContent>
                                        </Select>
                                        <Select onValueChange={handleSelectScript}>
                                            <SelectTrigger className="w-[140px] h-8 text-xs">
                                                <SelectValue placeholder="Load a script..." />
                                            </SelectTrigger>
                                            <SelectContent>
                                                {scripts.map(script => <SelectItem key={script.id} value={String(script.id)}>{script.name}</SelectItem>)}
                                            </SelectContent>
                                        </Select>
                                        <Button variant="ghost" size="sm" onClick={() => setShowHistory(!showHistory)}>
                                            <History className="h-4 w-4 mr-1" /> {showHistory ? 'Hide' : 'Show'}
                                        </Button>
                                    </div>
                                </div>
                                <div className="relative w-full rounded-md border" style={{ height: `${editorHeight}px` }}>
                                    <Editor theme={themeType === 'light' ? 'vs-light' : 'vs-dark'} language={scriptLanguage} value={command} onChange={(value) => setCommand(value || '')} options={{ minimap: { enabled: false }, scrollbar: { vertical: 'auto', horizontal: 'auto' }, wordWrap: 'on', lineNumbers: 'off', glyphMargin: false, folding: false, lineDecorationsWidth: 0, lineNumbersMinChars: 0, renderLineHighlight: 'none' }} />
                                </div>
                                <div onMouseDown={handleResizeMouseDown} className="w-full h-2 cursor-ns-resize bg-muted hover:bg-muted-foreground/20 transition-colors rounded-b-md" title="Drag to resize editor" />
                                {showHistory && (
                                    <Card className="mt-2 max-h-64 overflow-y-auto">
                                        <CardContent className="p-2">
                                            {commandHistory.length > 0 ? (
                                                <>
                                                    <div className="flex justify-end mb-1">
                                                        <Button variant="link" size="sm" className="text-destructive" onClick={handleClearHistory}>Clear All</Button>
                                                    </div>
                                                    {commandHistory.map((cmd, index) => (
                                                        <div key={index} className="flex items-center justify-between p-1 hover:bg-muted rounded group">
                                                            <div onClick={() => { setCommand(cmd); setShowHistory(false); }} className="cursor-pointer text-sm truncate flex-grow" title={cmd}>{cmd}</div>
                                                            <Button variant="ghost" size="icon" className="h-6 w-6 ml-2 text-destructive opacity-0 group-hover:opacity-100" onClick={(e) => { e.stopPropagation(); handleDeleteFromHistory(index); }}>
                                                                <X className="h-4 w-4" />
                                                            </Button>
                                                        </div>
                                                    ))}
                                                </>
                                            ) : <p className="text-sm text-muted-foreground text-center p-4">No history yet.</p>}
                                        </CardContent>
                                    </Card>
                                )}
                            </div>
                            <div className="flex space-x-2">
                                <Button onClick={handleSendCommand} disabled={selectedVps.size === 0 || command.trim() === '' || isLoading}>
                                    {isLoading ? 'Executing...' : 'Run Command'}
                                </Button>
                                <Button variant="secondary" onClick={() => setShowSaveModal(true)} disabled={command.trim() === ''}>Save as Script</Button>
                                {isLoading && currentBatchCommandId && <Button variant="destructive" onClick={handleTerminateCommand}>Terminate</Button>}
                            </div>
                            {error && <div className="p-2 bg-destructive/10 text-destructive border border-destructive/20 rounded-md text-sm">{error}</div>}
                            
                            <div className="mt-4 border-t pt-4">
                                <div className="flex justify-between items-center mb-2">
                                    <h2 className="text-lg font-semibold">Live Output</h2>
                                    <div className="flex items-center space-x-4">
                                        <div className="flex items-center space-x-2">
                                            <Label htmlFor="timestamps-switch" className="text-sm font-medium">Timestamps</Label>
                                            <Switch id="timestamps-switch" checked={showMetadata} onCheckedChange={setShowMetadata} />
                                        </div>
                                        <div className="flex items-center space-x-2">
                                            <Button variant={activeView === 'all' ? 'secondary' : 'ghost'} size="sm" onClick={() => setActiveView('all')}>Aggregated</Button>
                                            <Button variant={activeView === 'per-server' ? 'secondary' : 'ghost'} size="sm" onClick={() => setActiveView('per-server')}>Per-Server</Button>
                                        </div>
                                    </div>
                                </div>
                                <div className={`bg-muted/50 text-foreground p-4 rounded-md font-mono text-sm h-96 overflow-y-auto ${!showMetadata ? 'hide-metadata' : ''}`}>
                                    {activeView === 'all' && (
                                        <>
                                            {generalOutput.map((line, index) => <div key={`general-${index}`} style={{ whiteSpace: 'pre-wrap' }} dangerouslySetInnerHTML={{ __html: line }} />)}
                                            {aggregatedLogs.map((item, index) => (
                                                <div key={`agg-${index}`} style={{ whiteSpace: 'pre-wrap' }}>
                                                    <span className="text-cyan-400 mr-2">[{item.vpsName}]</span>
                                                    <span dangerouslySetInnerHTML={{ __html: item.log }} />
                                                </div>
                                            ))}
                                            {generalOutput.length === 0 && aggregatedLogs.length === 0 && <p>Command output will appear here...</p>}
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
                                                            <summary className="cursor-pointer font-semibold text-muted-foreground">{vpsName} - <span className="text-yellow-400">Pending...</span></summary>
                                                        </details>
                                                    );
                                                }
                                                const statusColor = data.status.toLowerCase().includes('success') || (data.exitCode === 0) ? 'text-green-400' : data.status.toLowerCase().includes('fail') || (typeof data.exitCode === 'number' && data.exitCode > 0) ? 'text-red-400' : 'text-yellow-400';
                                                return (
                                                    <details key={vpsId} className="mb-2" open>
                                                        <summary className="cursor-pointer font-semibold">{data.name} - <span className={statusColor}>{data.status} (Exit: {data.exitCode ?? 'N/A'})</span></summary>
                                                        <div className="pl-4 mt-2 border-l-2 border-border">
                                                            {data.logs.map((log, index) => <div key={index} style={{ whiteSpace: 'pre-wrap' }} dangerouslySetInnerHTML={{ __html: log }} />)}
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
                    </CardContent>
                </Card>
            </div>

            <SaveScriptModal isOpen={showSaveModal} onClose={() => setShowSaveModal(false)} onSave={handleSaveScript} initialCommand={command} />
        </div>
    );
};

export default BatchCommandPage;